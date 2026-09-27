; Yükseltilmiş NSIS kurucusu dosyaları kopyaladıktan sonra kök izinlerini kurar.
!macro NSIS_HOOK_POSTINSTALL
  ClearErrors
  ExecWait '"$INSTDIR\kalfa-yalitim.exe" kur' $0
  ${If} ${Errors}
    DetailPrint "Kalfa yalıtımı kurulamadı. Ayarlar'dan Onar'ı kullanın."
  ${ElseIf} $0 != 0
    DetailPrint "Kalfa yalıtımı kurulamadı (çıkış kodu $0). Ayarlar'dan Onar'ı kullanın."
  ${EndIf}
!macroend

; Yardımcı henüz yerindeyken kök izinlerini ve ortak araç izinlerini geri alır.
!macro NSIS_HOOK_PREUNINSTALL
  ${If} ${FileExists} "$INSTDIR\kalfa-yalitim.exe"
    ClearErrors
    ExecWait '"$INSTDIR\kalfa-yalitim.exe" kaldir' $0
    ${If} ${Errors}
      CreateDirectory "$PROGRAMFILES64\Kalfa-Kurtarma"
      ClearErrors
      CopyFiles /SILENT "$INSTDIR\kalfa-yalitim.exe" "$PROGRAMFILES64\Kalfa-Kurtarma"
      ${If} ${Errors}
        MessageBox MB_OK|MB_ICONEXCLAMATION "Kalfa yalıtım izinleri geri alınamadı ve kurtarma yardımcısı kopyalanamadı. Kurucuyu yeniden çalıştırarak yardımcıyı geri yükleyin." /SD IDOK
      ${Else}
        MessageBox MB_OK|MB_ICONEXCLAMATION "Kalfa yalıtım izinleri geri alınamadı. Kaldırmadan sonra yönetici PowerShell'de geri-al.ps1 betiğini çalıştırın." /SD IDOK
      ${EndIf}
    ${ElseIf} $0 != 0
      CreateDirectory "$PROGRAMFILES64\Kalfa-Kurtarma"
      ClearErrors
      CopyFiles /SILENT "$INSTDIR\kalfa-yalitim.exe" "$PROGRAMFILES64\Kalfa-Kurtarma"
      ${If} ${Errors}
        MessageBox MB_OK|MB_ICONEXCLAMATION "Kalfa yalıtım izinleri geri alınamadı (çıkış kodu $0) ve kurtarma yardımcısı kopyalanamadı. Kurucuyu yeniden çalıştırarak yardımcıyı geri yükleyin." /SD IDOK
      ${Else}
        MessageBox MB_OK|MB_ICONEXCLAMATION "Kalfa yalıtım izinleri geri alınamadı (çıkış kodu $0). Kaldırmadan sonra yönetici PowerShell'de geri-al.ps1 betiğini çalıştırın." /SD IDOK
      ${EndIf}
    ${EndIf}
  ${Else}
    MessageBox MB_OK|MB_ICONEXCLAMATION "Kalfa yalıtım yardımcısı bulunamadı. Kaldırma bitince BT departmanınızdan kalan izinleri incelemesini isteyin." /SD IDOK
  ${EndIf}
!macroend
