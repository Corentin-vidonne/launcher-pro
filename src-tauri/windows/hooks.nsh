; Étapes ajoutées à l'installeur NSIS de Launcher Pro.

; La désinstallation retire aussi le démarrage avec Windows, sinon la session tenterait encore
; de lancer un programme qui n'existe plus.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Launcher Pro"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Launcher Pro"
!macroend
