; Vektor Lite: the memory cleaner is the reason to install it, so setup turns it on.
; The VektorLiteMemory service runs from the app's own exe, so stop it before the
; file is replaced on a reinstall...
!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec '"$SYSDIR\sc.exe" stop VektorLiteMemory'
  Pop $0
  Sleep 2000
!macroend

; ...then install (or restart) it with the low-RAM rules. The installer is already
; elevated (per-machine), so the exe's admin manifest does not prompt again.
!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec '"$INSTDIR\${MAINBINARYNAME}.exe" --install-memory-service'
  Pop $0
!macroend

; Remove the service before the files are deleted on uninstall.
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec '"$SYSDIR\sc.exe" stop VektorLiteMemory'
  Pop $0
  Sleep 1500
  nsExec::Exec '"$SYSDIR\sc.exe" delete VektorLiteMemory'
  Pop $0
!macroend
