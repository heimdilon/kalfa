//! P10: yalıtılmış süreç geliştiricinin penceresine ileti gönderebiliyor,
//! panoyu açabiliyor ve başka pencereleri bulabiliyor mu? `ui-dene <hwnd>`
//! yalıtımın içinde çalışır; `pencere_ac` geliştirici tarafında gizli bir
//! pencere açıp gelen iletiyi sayar.

use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, IsClipboardFormatAvailable, OpenClipboard,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, MSG, PostMessageW,
    RegisterClassW, TranslateMessage, WM_APP, WNDCLASSW, WS_OVERLAPPED,
};

use crate::win::genis;

const ILETI: u32 = WM_APP + 7;
pub static GELEN: AtomicU32 = AtomicU32::new(0);

/// Yalıtımın içinde: sonuçları `anahtar=değer` satırları olarak yazar.
pub fn dene(argumanlar: &[String]) -> i32 {
    let hwnd = argumanlar
        .first()
        .and_then(|s| s.parse::<isize>().ok())
        .unwrap_or(0) as HWND;
    // SAFETY: Win32 çağrıları; tutamaçlar yalnızca sınanır.
    unsafe {
        let gonderildi = PostMessageW(hwnd, ILETI, 42, 0) != 0;
        println!(
            "ileti={}",
            if gonderildi {
                "gonderildi"
            } else {
                "engellendi"
            }
        );
        let pano = OpenClipboard(null_mut()) != 0;
        if pano {
            let metin = IsClipboardFormatAvailable(13) != 0; // CF_UNICODETEXT
            CloseClipboard();
            println!("pano=acildi metin_var={metin}");
        } else {
            println!("pano=engellendi");
        }
        let gorev_cubugu = !FindWindowW(genis("Shell_TrayWnd").as_ptr(), null()).is_null();
        println!(
            "pencere_bulma={}",
            if gorev_cubugu { "buldu" } else { "bulamadi" }
        );
    }
    0
}

unsafe extern "system" fn pencere_islevi(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if m == ILETI && w == 42 {
        GELEN.fetch_add(1, Ordering::SeqCst);
        return 0;
    }
    // SAFETY: varsayılan işleyici.
    unsafe { DefWindowProcW(h, m, w, l) }
}

/// Geliştirici tarafında gizli bir üst düzey pencere açar; tutamacını döndürür.
/// İleti döngüsü ayrı bir iş parçacığında sürer.
pub fn pencere_ac() -> isize {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // SAFETY: sınıf ve pencere bu iş parçacığına aittir.
        unsafe {
            let sinif = genis("KalfaPrototipPencere");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(pencere_islevi),
                hInstance: GetModuleHandleW(null()),
                lpszClassName: sinif.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let h = CreateWindowExW(
                0,
                sinif.as_ptr(),
                genis("Kalfa prototip").as_ptr(),
                WS_OVERLAPPED,
                0,
                0,
                10,
                10,
                null_mut(),
                null_mut(),
                wc.hInstance,
                null(),
            );
            let _ = tx.send(h as isize);
            let mut m: MSG = std::mem::zeroed();
            while GetMessageW(&mut m, null_mut(), 0, 0) > 0 {
                TranslateMessage(&m);
                DispatchMessageW(&m);
            }
        }
    });
    rx.recv().unwrap_or(0)
}
