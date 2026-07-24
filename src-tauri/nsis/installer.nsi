; media-to-doc NSIS installer (W14-C B + v1.4.0 bump + W14-G+ D 盘默认 + v1.4.1 v0.1.0 badge fix)
; Uses system NSIS 3.12, bypasses Tauri bundler GitHub TLS issue
; W14-G+: Tauri `windows.nsis.template` 字段把本文件拷到 target/release/nsis/x64/installer.nsi,
;         makensis working dir 改为 target/release/nsis/x64/(上 2 层到 target/release/);
;         OutFile 必须为 "nsis-output.exe"(Tauri 期望的固定名,会 fs::rename 到 bundle/nsis/<product>_<version>_<arch>-setup.exe);
;         File 路径使用 ..\..\ 相对路径;
;         MUI_PAGE_LICENSE 删除(原 LICENSE.txt 不在新 working dir,无 license 页)

!define PRODUCT_NAME "media-to-doc"
!define PRODUCT_VERSION "1.4.1"
!define PRODUCT_PUBLISHER "Duanyi"
!define PRODUCT_DIR_REGKEY "Software\Microsoft\Windows\CurrentVersion\App Paths\media-to-doc-ui.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

SetCompressor lzma

; MUI 2
!include "MUI2.nsh"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "nsis-output.exe"
InstallDir "D:\Program Files\MediaToDoc"
InstallDirRegKey HKLM "${PRODUCT_DIR_REGKEY}" ""
RequestExecutionLevel admin

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"

Section "Install"
  SetOutPath "$INSTDIR"
  File "..\..\media-to-doc-ui.exe"
  CreateDirectory "$INSTDIR"

  WriteRegStr HKLM "${PRODUCT_DIR_REGKEY}" "" "$INSTDIR\media-to-doc-ui.exe"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninst.exe"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\media-to-doc-ui.exe"
  WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoRepair" 1

  WriteUninstaller "$INSTDIR\uninst.exe"

  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" "$INSTDIR\media-to-doc-ui.exe"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall.lnk" "$INSTDIR\uninst.exe"
  CreateShortCut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\media-to-doc-ui.exe"

  ; .mtdproj file association
  WriteRegStr HKLM "Software\Classes\.mtdproj" "" "MediaToDoc.Project"
  WriteRegStr HKLM "Software\Classes\MediaToDoc.Project" "" "media-to-doc Project"
  WriteRegStr HKLM "Software\Classes\MediaToDoc.Project\DefaultIcon" "" "$INSTDIR\media-to-doc-ui.exe,0"
  WriteRegStr HKLM "Software\Classes\MediaToDoc.Project\shell\open\command" "" '"$INSTDIR\media-to-doc-ui.exe" "%1"'
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\media-to-doc-ui.exe"
  Delete "$INSTDIR\uninst.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall.lnk"
  RMDir "$SMPROGRAMS\${PRODUCT_NAME}"
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"

  DeleteRegKey HKLM "Software\Classes\.mtdproj"
  DeleteRegKey HKLM "Software\Classes\MediaToDoc.Project"

  DeleteRegKey HKLM "${PRODUCT_UNINST_KEY}"
  DeleteRegKey HKLM "${PRODUCT_DIR_REGKEY}"
SectionEnd
