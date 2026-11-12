; The VektorMemory service runs from vektor.exe itself, so the file is locked while
; the service is up. Stop it before an install or update replaces the file...
!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec '"$SYSDIR\sc.exe" stop VektorMemory'
  Pop $0
  Sleep 2000
!macroend

; ...and bring it back afterwards. Fails harmlessly when the service is not installed.
!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec '"$SYSDIR\sc.exe" start VektorMemory'
  Pop $0
!macroend

; Remove the service before the files are deleted on uninstall.
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec '"$SYSDIR\sc.exe" stop VektorMemory'
  Pop $0
  Sleep 1500
  nsExec::Exec '"$SYSDIR\sc.exe" delete VektorMemory'
  Pop $0
!macroend
