//! seccomp filtreleri (aşama 2 tasarımı §8.2). bwrap'a iki filtre verilir
//! (`--add-seccomp-fd`); çekirdek her sistem çağrısında ikisini de çalıştırır
//! ve en kısıtlayıcı sonucu uygular.
//!
//! - **Engel filtresi** (`seccompiler`): aşağıdaki çağrılar `EPERM` döner.
//!   Mimari tutmazsa (örneğin x86_64'te 32 bit `int 0x80`) süreç öldürülür.
//! - **`ENOSYS` filtresi** (elle yazılmış): `clone3`'ün bayrakları bir yapının
//!   içindedir ve filtrelenemez; `ENOSYS` alan libc `clone`'a geri düşer, onun
//!   bayrakları engel filtresinde denetlenir. x86_64'te x32 çağrıları
//!   (numarasında `0x40000000` biti olanlar) da `ENOSYS` döner: aynı mimari
//!   değeriyle gelip numara listesini atlatabilirlerdi.

use std::collections::BTreeMap;

use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen as Uzunluk, SeccompCmpOp as Islem,
    SeccompCondition, SeccompFilter, SeccompRule, TargetArch, sock_filter,
};

/// Her iki profilde de koşulsuz engellenenler.
const ENGELLI: &[i64] = &[
    libc::SYS_io_uring_setup,
    libc::SYS_io_uring_enter,
    libc::SYS_io_uring_register,
    libc::SYS_keyctl,
    libc::SYS_add_key,
    libc::SYS_request_key,
    libc::SYS_bpf,
    libc::SYS_perf_event_open,
    libc::SYS_userfaultfd,
    libc::SYS_ptrace,
    libc::SYS_pidfd_getfd,
];

/// Ad alanı bayrakları. `CLONE_NEWTIME` `clone`'da çıkış sinyali baytıyla
/// çakıştığı için yalnızca `unshare`'de denetlenir.
const AD_ALANI_BAYRAKLARI: &[u64] = &[
    libc::CLONE_NEWUSER as u64,
    libc::CLONE_NEWNS as u64,
    libc::CLONE_NEWPID as u64,
    libc::CLONE_NEWNET as u64,
    libc::CLONE_NEWIPC as u64,
    libc::CLONE_NEWUTS as u64,
    libc::CLONE_NEWCGROUP as u64,
];
const CLONE_NEWTIME: u64 = 0x80;

#[derive(Debug, thiserror::Error)]
#[error("seccomp filtresi üretilemedi: {0}")]
pub struct FiltreHatasi(String);

fn hata(e: impl std::fmt::Display) -> FiltreHatasi {
    FiltreHatasi(e.to_string())
}

pub(crate) fn mimari() -> Result<TargetArch, FiltreHatasi> {
    TargetArch::try_from(std::env::consts::ARCH).map_err(hata)
}

fn kosul(
    arguman: u8,
    uzunluk: Uzunluk,
    islem: Islem,
    deger: u64,
) -> Result<SeccompCondition, FiltreHatasi> {
    SeccompCondition::new(arguman, uzunluk, islem, deger).map_err(hata)
}

/// Bir bayrak kümesinden biri bile verilmişse eşleşen kurallar.
fn bayrak_kurallari(bayraklar: &[u64]) -> Result<Vec<SeccompRule>, FiltreHatasi> {
    bayraklar
        .iter()
        .map(|b| {
            SeccompRule::new(vec![kosul(0, Uzunluk::Qword, Islem::MaskedEq(*b), *b)?]).map_err(hata)
        })
        .collect()
}

/// `socket` çağrısında izin verilen adres aileleri (§8.2); gerisi engellenir.
/// İzin listesidir: ağ ad alanının kesmediği aileler (`AF_VSOCK` gibi) tek
/// tek sayılmak zorunda kalınmaz. Ağlı profilde `AF_INET` ve `AF_INET6`
/// açılabilir. `AF_NETLINK` yalnızca `NETLINK_ROUTE` (protokol 0) ile
/// açılabilir. `socketpair` serbesttir.
fn soket_kurallari(ag: bool) -> Result<Vec<SeccompRule>, FiltreHatasi> {
    let mut izinli = vec![libc::AF_NETLINK];
    if ag {
        izinli.extend([libc::AF_INET, libc::AF_INET6]);
    }
    let disinda = izinli
        .into_iter()
        .map(|aile| kosul(0, Uzunluk::Dword, Islem::Ne, aile as u64))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(vec![
        SeccompRule::new(disinda).map_err(hata)?,
        SeccompRule::new(vec![
            kosul(0, Uzunluk::Dword, Islem::Eq, libc::AF_NETLINK as u64)?,
            kosul(2, Uzunluk::Dword, Islem::Ne, libc::NETLINK_ROUTE as u64)?,
        ])
        .map_err(hata)?,
    ])
}

/// Engel filtresi: eşleşen çağrı `EPERM` döner.
pub(crate) fn engel_filtresi(ag: bool) -> Result<BpfProgram, FiltreHatasi> {
    let mut kurallar: BTreeMap<i64, Vec<SeccompRule>> =
        ENGELLI.iter().map(|c| (*c, Vec::new())).collect();
    let mut unshare = AD_ALANI_BAYRAKLARI.to_vec();
    unshare.push(CLONE_NEWTIME);
    kurallar.insert(libc::SYS_unshare, bayrak_kurallari(&unshare)?);
    kurallar.insert(libc::SYS_clone, bayrak_kurallari(AD_ALANI_BAYRAKLARI)?);
    kurallar.insert(libc::SYS_socket, soket_kurallari(ag)?);
    let filtre = SeccompFilter::new(
        kurallar,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        mimari()?,
    )
    .map_err(hata)?;
    BpfProgram::try_from(filtre).map_err(hata)
}

/// `BPF_LD | BPF_W | BPF_ABS`.
const BPF_LD_W_ABS: u16 = 0x20;
const BPF_JMP_JEQ_K: u16 = 0x05 | 0x10;
const BPF_JMP_JGE_K: u16 = 0x05 | 0x30;
const BPF_RET_K: u16 = 0x06;
/// `seccomp_data` içinde sistem çağrısı numarası ve mimari.
const NR: u32 = 0;
const ARCH: u32 = 4;
const AUDIT_ARCH_X86_64: u32 = 62 | 0x8000_0000 | 0x4000_0000;
const AUDIT_ARCH_AARCH64: u32 = 183 | 0x8000_0000 | 0x4000_0000;
const X32_BITI: u32 = 0x4000_0000;

fn komut(code: u16, jt: u8, jf: u8, k: u32) -> sock_filter {
    sock_filter { code, jt, jf, k }
}

/// `ENOSYS` filtresi. Mimari tutmazsa izin verir; o durumu engel filtresi
/// süreci öldürerek karşılar.
pub(crate) fn enosys_filtresi() -> Result<BpfProgram, FiltreHatasi> {
    let izin = libc::SECCOMP_RET_ALLOW;
    let enosys = libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32;
    let clone3 = libc::SYS_clone3 as u32;
    Ok(match mimari()? {
        TargetArch::x86_64 => vec![
            komut(BPF_LD_W_ABS, 0, 0, ARCH),
            komut(BPF_JMP_JEQ_K, 0, 3, AUDIT_ARCH_X86_64),
            komut(BPF_LD_W_ABS, 0, 0, NR),
            komut(BPF_JMP_JEQ_K, 2, 0, clone3),
            komut(BPF_JMP_JGE_K, 1, 0, X32_BITI),
            komut(BPF_RET_K, 0, 0, izin),
            komut(BPF_RET_K, 0, 0, enosys),
        ],
        TargetArch::aarch64 => vec![
            komut(BPF_LD_W_ABS, 0, 0, ARCH),
            komut(BPF_JMP_JEQ_K, 0, 2, AUDIT_ARCH_AARCH64),
            komut(BPF_LD_W_ABS, 0, 0, NR),
            komut(BPF_JMP_JEQ_K, 1, 0, clone3),
            komut(BPF_RET_K, 0, 0, izin),
            komut(BPF_RET_K, 0, 0, enosys),
        ],
        diger => return Err(FiltreHatasi(format!("desteklenmeyen mimari: {diger:?}"))),
    })
}

/// Programı bwrap'ın okuduğu biçime çevirir: `struct sock_filter` dizisi.
pub(crate) fn baytlar(program: &[sock_filter]) -> Vec<u8> {
    program
        .iter()
        .flat_map(|k| {
            let mut b = Vec::with_capacity(8);
            b.extend_from_slice(&k.code.to_ne_bytes());
            b.push(k.jt);
            b.push(k.jf);
            b.extend_from_slice(&k.k.to_ne_bytes());
            b
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iki_profilin_filtreleri_uretilir_ve_farklidir() {
        let agsiz = engel_filtresi(false).unwrap();
        let agli = engel_filtresi(true).unwrap();
        assert!(!agsiz.is_empty());
        assert_ne!(agsiz, agli);
        let enosys = enosys_filtresi().unwrap();
        assert_eq!(baytlar(&enosys).len(), enosys.len() * 8);
        // Son iki komut: izin ve ENOSYS.
        assert_eq!(enosys[enosys.len() - 2].k, libc::SECCOMP_RET_ALLOW);
        assert_eq!(
            enosys[enosys.len() - 1].k,
            libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32
        );
    }

    #[test]
    fn pidfd_getfd_engelli() {
        assert!(ENGELLI.contains(&libc::SYS_pidfd_getfd));
        let agsiz = engel_filtresi(false).unwrap();
        let agli = engel_filtresi(true).unwrap();
        assert!(!agsiz.is_empty());
        assert!(!agli.is_empty());
    }
}
