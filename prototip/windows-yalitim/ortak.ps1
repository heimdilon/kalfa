# Kalfa 2c prototipi: kur.ps1 ve geri-al.ps1'in ortak parçaları. Tek başına
# çalıştırılmaz.

$Yerel = Join-Path $env:LOCALAPPDATA 'KalfaPrototip'
$KurulumDosyasi = Join-Path $Yerel 'kurulum.json'
$ParolaDosyasi = Join-Path $Yerel 'parolalar.bin'
$Grup = 'KalfaPrototip'
$Agsiz = 'KalfaPrototipAgsiz'
$Agli = 'KalfaPrototipAgli'
$KuralAdi = 'KalfaPrototipAgsiz'
$UserListAnahtari = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon\SpecialAccounts\UserList'
$IzinVerilenHaklar = @('SeInteractiveLogonRight')
$ReddedilenHaklar = @(
    'SeDenyNetworkLogonRight',
    'SeDenyRemoteInteractiveLogonRight',
    'SeDenyBatchLogonRight',
    'SeDenyServiceLogonRight'
)

# Oturum açma haklari (LSA). Home surumunde secpol yok; dogrudan API.
if (-not ('KalfaLsa' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Security.Principal;

public static class KalfaLsa {
    [StructLayout(LayoutKind.Sequential)]
    struct LSA_UNICODE_STRING { public ushort Length; public ushort MaximumLength; public IntPtr Buffer; }
    [StructLayout(LayoutKind.Sequential)]
    struct LSA_OBJECT_ATTRIBUTES { public int Length; public IntPtr RootDirectory; public IntPtr ObjectName; public int Attributes; public IntPtr SecurityDescriptor; public IntPtr SecurityQualityOfService; }

    [DllImport("advapi32.dll")] static extern uint LsaOpenPolicy(IntPtr systemName, ref LSA_OBJECT_ATTRIBUTES attributes, int access, out IntPtr policy);
    [DllImport("advapi32.dll")] static extern uint LsaAddAccountRights(IntPtr policy, byte[] sid, LSA_UNICODE_STRING[] rights, int count);
    [DllImport("advapi32.dll")] static extern uint LsaRemoveAccountRights(IntPtr policy, byte[] sid, bool all, LSA_UNICODE_STRING[] rights, int count);
    [DllImport("advapi32.dll")] static extern uint LsaClose(IntPtr policy);
    [DllImport("advapi32.dll")] static extern int LsaNtStatusToWinError(uint status);

    static IntPtr Ac() {
        var a = new LSA_OBJECT_ATTRIBUTES();
        a.Length = Marshal.SizeOf(a);
        IntPtr h;
        uint r = LsaOpenPolicy(IntPtr.Zero, ref a, 0x000F0FFF, out h);
        if (r != 0) throw new Win32Exception(LsaNtStatusToWinError(r));
        return h;
    }
    static byte[] Sid(string sid) {
        var s = new SecurityIdentifier(sid);
        var b = new byte[s.BinaryLength];
        s.GetBinaryForm(b, 0);
        return b;
    }
    static LSA_UNICODE_STRING[] Hak(string hak) {
        var u = new LSA_UNICODE_STRING();
        u.Buffer = Marshal.StringToHGlobalUni(hak);
        u.Length = (ushort)(hak.Length * 2);
        u.MaximumLength = (ushort)(hak.Length * 2 + 2);
        return new[] { u };
    }
    public static void Ekle(string sid, string hak) {
        IntPtr h = Ac();
        try {
            uint r = LsaAddAccountRights(h, Sid(sid), Hak(hak), 1);
            if (r != 0) throw new Win32Exception(LsaNtStatusToWinError(r));
        } finally { LsaClose(h); }
    }
    public static void Kaldir(string sid, string hak) {
        IntPtr h = Ac();
        try {
            uint r = LsaRemoveAccountRights(h, Sid(sid), false, Hak(hak), 1);
            // STATUS_OBJECT_NAME_NOT_FOUND: hak zaten yok.
            if (r != 0 && r != 0xC0000034) throw new Win32Exception(LsaNtStatusToWinError(r));
        } finally { LsaClose(h); }
    }
}
'@
}

Add-Type -AssemblyName System.Security

function Kurulumu-Oku {
    if (-not (Test-Path $KurulumDosyasi)) { return $null }
    Get-Content -Raw -Encoding UTF8 $KurulumDosyasi | ConvertFrom-Json
}

function Kurulumu-Yaz($kurulum) {
    New-Item -ItemType Directory -Force $Yerel | Out-Null
    $kurulum | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 $KurulumDosyasi
}

# İzinler .NET ile verilir: icacls hiçbir hesaba karşılık gelmeyen SID'leri
# (ortak okuma SID'i gibi) kabul etmiyor.
function Izin-Ver([string]$yol, [string[]]$sidler) {
    $d = Get-Item -LiteralPath $yol -Force
    $acl = $d.GetAccessControl('Access')
    foreach ($s in $sidler) {
        $sid = New-Object Security.Principal.SecurityIdentifier $s
        $kural = New-Object Security.AccessControl.FileSystemAccessRule(
            $sid, 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
        $acl.AddAccessRule($kural)
    }
    $d.SetAccessControl($acl)
}

function Izin-Kaldir([string]$yol, [string[]]$sidler) {
    $d = Get-Item -LiteralPath $yol -Force
    $acl = $d.GetAccessControl('Access')
    foreach ($s in $sidler) {
        $acl.PurgeAccessRules((New-Object Security.Principal.SecurityIdentifier $s))
    }
    $d.SetAccessControl($acl)
}

# C:\ kökünün DACL'i SetFileSecurityW ile yazılır: .NET'in SetAccessControl'ü
# (SetNamedSecurityInfo) değişiklikten sonra bütün sürücüde mirası yeniden
# yayar. SetFileSecurityW yalnızca kökü değiştirir; eklenen izin de miras
# verilmez.
if (-not ('KalfaKok' -as [type])) {
    Add-Type -TypeDefinition @'
using System.ComponentModel;
using System.Runtime.InteropServices;
public static class KalfaKok {
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool SetFileSecurityW(string yol, int bilgi, byte[] tanimlayici);
    public static void DaclYaz(string yol, byte[] tanimlayici) {
        // DACL_SECURITY_INFORMATION
        if (!SetFileSecurityW(yol, 4, tanimlayici)) throw new Win32Exception(Marshal.GetLastWin32Error());
    }
}
'@
}

$KokHaklari = 'ListDirectory, ReadAttributes, Traverse, Synchronize'

# SetFileSecurityW, SE_DACL_AUTO_INHERITED (AI) bayrağını ancak
# SE_DACL_AUTO_INHERIT_REQ ile birlikte verilirse korur; yoksa kökün DACL'i
# "D:PAI"dan "D:P"ye düşer (ölçüldü). Denetim sözcüğü tanımlayıcının 2-3.
# baytlarıdır: 0x0100 AUTO_INHERIT_REQ, 0x0400 AUTO_INHERITED.
function Kok-Dacl-Yaz($acl, [bool]$otomatikMiras) {
    $b = $acl.GetSecurityDescriptorBinaryForm()
    if ($otomatikMiras) { $b[3] = [byte]($b[3] -bor 0x05) }
    [KalfaKok]::DaclYaz('C:\', $b)
}

function Kok-Izni-Ver([string]$sid) {
    $ai = (Get-Acl -LiteralPath 'C:\').Sddl -match 'D:P?AI'
    $acl = (Get-Item -LiteralPath 'C:\' -Force).GetAccessControl('Access')
    $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule(
        (New-Object Security.Principal.SecurityIdentifier $sid), $KokHaklari, 'None', 'None', 'Allow')))
    Kok-Dacl-Yaz $acl $ai
}

# Windows'un kök DACL'i AI bayrağını taşır; ilk sürüm bu bayrağı düşürdüğü
# için kaldırırken her durumda geri konur.
function Kok-Izni-Kaldir([string]$sid) {
    $acl = (Get-Item -LiteralPath 'C:\' -Force).GetAccessControl('Access')
    $acl.PurgeAccessRules((New-Object Security.Principal.SecurityIdentifier $sid))
    Kok-Dacl-Yaz $acl $true
}

function Users-Grubu {
    Get-LocalGroup -SID 'S-1-5-32-545'
}
