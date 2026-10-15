; Remove the VektorMemory service before the files are deleted on uninstall.
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec '"$SYSDIR\sc.exe" stop VektorMemory'
  Sleep 1500
  nsExec::Exec '"$SYSDIR\sc.exe" delete VektorMemory'
!macroend
