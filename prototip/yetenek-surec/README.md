# Kalfa Yetenek Programı Hat 03: Süreç ve Terminal Prototipi (G2a)

Hat 03 (`2026-10-20-kalfa-yetenek-03-surec-terminal.md`) Görev 1 ve Görev 2
kapsamındaki **G2a** deney kapısı. Ürün kodu değildir; Kalfa'nın workspace'ine
girmez. Ölçüm sonuçları ve mimari kararlar:
[surec-deneyi.md](../../docs/superpowers/bulgular/yetenek-programi/surec-deneyi.md).

## Koşum

### Windows (yönetici gerekmez)

```powershell
$env:TEMP = "C:\Users\kor_a\kalfa-hat-03-tmp"
$env:TMP = "C:\Users\kor_a\kalfa-hat-03-tmp"
cargo run --offline -- olc
```

### Linux (WSL Ubuntu 24.04, `kalfatest`)

```bash
cargo run --offline --manifest-path prototip/yetenek-surec/Cargo.toml \
  --target-dir /tmp/kalfa-hat-03-prototip-target -- olc
rm -rf /tmp/kalfa-hat-03-prototip-target
```

## Kapsam

- `src/kilit_deneyi.rs`: Görev 1 (`KaynakKilitleri` genel yazma kilidi, bekleyen
  `dosya_yaz`, `kilitsiz: false` kontrol çağrısında kilitlenme/deadlock yeniden
  üretimi, erken dönüşte `.git/config` koruma kaçışı ve `sahipli_al` +
  `kilitsiz: true` + `komut_sonrasi_denetim` çözümü).
- `src/win_deneyi.rs`: Görev 2 Windows (`AppContainer` + `Job Object` içinde
  borulu stdin/stdout/EOF, `ConPTY` `CreatePseudoConsole` + `ResizePseudoConsole`
  + `ClosePseudoConsole`, `\x03` `Ctrl+C` `CTRL_C_EVENT`, torun süreç ağacı
  sonlandırma, 1 MiB dolu çıktı taşma sınırı ve profil/dizin temizliği).
- `src/linux_deneyi.rs`: Görev 2 Linux (`bwrap` + `seccomp` + `PR_SET_CHILD_SUBREAPER`
  içinde borulu stdin/stdout/EOF, `openpty` + `/usr/bin/setsid -c` ile `TIOCSCTTY`
  ve `\x03` `SIGINT`, `TIOCSWINSZ` boyutlandırma, torun süreç ağacı sonlandırma
  ve 1 MiB dolu çıktı taşma sınırı).
