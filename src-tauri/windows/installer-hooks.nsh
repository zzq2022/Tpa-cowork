; Install VC++ runtime and atomically install the bundled agent Python runtime.
; The archive remains available when extraction fails so the Rust runtime can
; retry on first launch.

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Installing Microsoft Visual C++ 2015-2022 Redistributable..."
  ExecWait '"$INSTDIR\resources\vc_redist.x64.exe" /install /quiet /norestart' $0
  ${If} $0 = 0
    DetailPrint "VC++ Redistributable installed."
  ${ElseIf} $0 = 1638
    DetailPrint "VC++ Redistributable already up to date."
  ${ElseIf} $0 = 3010
    DetailPrint "VC++ Redistributable installed (reboot recommended after TPA CoWork setup)."
  ${Else}
    DetailPrint "VC++ Redistributable installation failed (exit code $0)."
    MessageBox MB_OK|MB_ICONEXCLAMATION "VC++ Redistributable installation failed (exit code $0). TPA CoWork may not start until the runtime is installed from https://aka.ms/vs/17/release/vc_redist.x64.exe"
  ${EndIf}
  Delete "$INSTDIR\resources\vc_redist.x64.exe"

  StrCpy $1 "$INSTDIR\resources\agent-venv.zip"
  IfFileExists "$1" agent_venv_zip_found 0
  StrCpy $1 "$INSTDIR\agent-venv.zip"
  IfFileExists "$1" agent_venv_zip_found agent_venv_zip_missing

  agent_venv_zip_found:
  DetailPrint "Extracting agent-venv from $1..."
  StrCpy $2 "$INSTDIR\.agent-venv-staging"
  StrCpy $3 "$INSTDIR\.agent-venv-backup"
  RMDir /r "$2"
  RMDir /r "$3"
  CreateDirectory "$2"

  StrCpy $4 "$WINDIR\Sysnative\tar.exe"
  IfFileExists "$4" agent_venv_tar_ready 0
  StrCpy $4 "$WINDIR\System32\tar.exe"
  IfFileExists "$4" agent_venv_tar_ready 0
  StrCpy $4 ""

  agent_venv_tar_ready:
  ${If} $4 != ""
    nsExec::ExecToLog '"$4" -xf "$1" -C "$2"'
    Pop $0
  ${Else}
    StrCpy $0 1
  ${EndIf}
  ${If} $0 != 0
    DetailPrint "tar.exe unavailable or failed; using PowerShell Expand-Archive."
    nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "Expand-Archive -LiteralPath ''$1'' -DestinationPath ''$2'' -Force"'
    Pop $0
  ${EndIf}

  IfFileExists "$2\agent-venv\Scripts\python.exe" agent_venv_staged_ok agent_venv_failed

  agent_venv_staged_ok:
  IfFileExists "$INSTDIR\agent-venv\*.*" agent_venv_backup_existing agent_venv_promote

  agent_venv_backup_existing:
  Rename "$INSTDIR\agent-venv" "$3"
  IfErrors agent_venv_failed

  agent_venv_promote:
  Rename "$2\agent-venv" "$INSTDIR\agent-venv"
  IfErrors agent_venv_restore
  FileOpen $4 "$INSTDIR\agent-venv\.tpa-cowork-venv-complete" w
  FileWrite $4 "complete"
  FileClose $4
  RMDir /r "$3"
  Delete "$1"
  Delete "$INSTDIR\resources\agent-venv.zip"
  Delete "$INSTDIR\agent-venv.zip"
  Goto agent_venv_done

  agent_venv_restore:
  IfFileExists "$3\Scripts\python.exe" 0 agent_venv_failed
  Rename "$3" "$INSTDIR\agent-venv"
  Goto agent_venv_failed

  agent_venv_failed:
  RMDir /r "$2"
  DetailPrint "agent-venv extraction failed (exit code $0); archive retained for runtime retry."
  MessageBox MB_OK|MB_ICONEXCLAMATION "Bundled Python extraction failed. TPA CoWork will retry on first launch. Re-run the installer if office or skill scripts remain unavailable."
  Goto agent_venv_done

  agent_venv_zip_missing:
  DetailPrint "agent-venv.zip not found; skipping Python environment extraction."

  agent_venv_done:
  RMDir "$INSTDIR\resources"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Removing Python environment (agent-venv)..."
  RMDir /r "$INSTDIR\agent-venv"
  RMDir /r "$INSTDIR\.agent-venv-staging"
  RMDir /r "$INSTDIR\.agent-venv-backup"
  Delete "$INSTDIR\agent-venv.zip"
  Delete "$INSTDIR\resources\agent-venv.zip"

  ${If} $DeleteAppData == "1"
    DetailPrint "Removing user data directory ($PROFILE\.tpa-cowork)..."
    RMDir /r "$PROFILE\.tpa-cowork"
    RMDir /r "$PROFILE\.hope-agent"
  ${Else}
    MessageBox MB_YESNO|MB_ICONQUESTION "是否同时清除所有用户数据与历史配置？$\r$\n$\r$\n删除路径: $PROFILE\.tpa-cowork$\r$\n（包含对话历史、记忆库及 API 密钥等。注意：此操作不可恢复！）" IDNO skip_user_data_cleanup
      DetailPrint "Removing user data directory ($PROFILE\.tpa-cowork)..."
      RMDir /r "$PROFILE\.tpa-cowork"
      RMDir /r "$PROFILE\.hope-agent"
    skip_user_data_cleanup:
  ${EndIf}
!macroend

