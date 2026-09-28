# Kalfa: Windows yalıtımında (AppContainer) Python uyumluluğu (A16).
#
# 1. Python 3.13+ `os.mkdir(yol, 0o700)`'ü yalnızca sahibe açık bir DACL'e
#    çevirir. AppContainer'daki süreç o DACL'de olmadığı için kendi açtığı
#    geçici dizine yazamaz (pip, venv, tempfile). Kapta geçici dizin zaten
#    çalışma alanına özeldir; 0o700 devralınan izinle açılır.
# 2. `GetFinalPathNameByHandleW` DOS adıyla kapta "Erişim engellendi" verir
#    (Mount Manager). NT adıyla çalışır; NT adı sürücü köklerinin NT
#    adlarıyla DOS adına çevrilir. Böylece `os.path.realpath` doğru çalışır.
import os
import sys

if sys.platform == "win32":
    import ctypes
    import ntpath
    from ctypes import wintypes

    _kalfa_mkdir = os.mkdir

    def _mkdir(path, mode=0o777, *, dir_fd=None):
        if mode == 0o700:
            mode = 0o777
        return _kalfa_mkdir(path, mode, dir_fd=dir_fd)

    os.mkdir = _mkdir

    _k32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _k32.CreateFileW.restype = wintypes.HANDLE
    _k32.CreateFileW.argtypes = [
        wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, wintypes.LPVOID,
        wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE,
    ]
    _k32.GetFinalPathNameByHandleW.argtypes = [
        wintypes.HANDLE, wintypes.LPWSTR, wintypes.DWORD, wintypes.DWORD,
    ]
    _k32.CloseHandle.argtypes = [wintypes.HANDLE]
    _GECERSIZ = wintypes.HANDLE(-1).value
    _VOLUME_NAME_NT = 2
    _SURUCULER = None
    _kalfa_getfinal = ntpath._getfinalpathname

    def _nt_adi(yol):
        h = _k32.CreateFileW(yol, 0, 7, None, 3, 0x02000000, None)
        if h in (None, _GECERSIZ):
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            boyut = 512
            while True:
                tampon = ctypes.create_unicode_buffer(boyut)
                n = _k32.GetFinalPathNameByHandleW(h, tampon, boyut, _VOLUME_NAME_NT)
                if n == 0:
                    raise ctypes.WinError(ctypes.get_last_error())
                if n < boyut:
                    return tampon.value
                boyut = n + 1
        finally:
            _k32.CloseHandle(h)

    def _suruculer():
        global _SURUCULER
        if _SURUCULER is None:
            eslesme = []
            maske = _k32.GetLogicalDrives()
            for i in range(26):
                if maske & (1 << i):
                    harf = chr(ord("A") + i) + ":"
                    try:
                        eslesme.append((_nt_adi(harf + "\\").rstrip("\\") + "\\", harf))
                    except OSError:
                        pass
            _SURUCULER = sorted(eslesme, key=lambda e: -len(e[0]))
        return _SURUCULER

    def _getfinalpathname(yol):
        try:
            return _kalfa_getfinal(yol)
        except OSError as hata:
            if getattr(hata, "winerror", None) != 5:
                raise
        nt = _nt_adi(os.fspath(yol))
        for onek, harf in _suruculer():
            if (nt + "\\").lower().startswith(onek.lower()):
                return "\\\\?\\" + harf + "\\" + nt[len(onek):]
        raise OSError(5, "Erişim engellendi", yol)

    ntpath._getfinalpathname = _getfinalpathname
