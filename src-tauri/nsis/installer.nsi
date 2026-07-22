; media-to-doc NSIS installer (W14-C B + v1.4.0 bump)
; Uses system NSIS 3.12, bypasses Tauri bundler GitHub TLS issue

!define PRODUCT_NAME "media-to-doc"
!define PRODUCT_VERSION "1.4.0"
!define PRODUCT_PUBLISHER "Duanyi"
!define PRODUCT_DIR_REGKEY "Software\Microsoft\Windows\CurrentVersion\App Paths\media-to-doc-ui.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

SetCompressor lzma

; MUI 2
!include "MUI2.nsh"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "..\target\release\bundle\nsis\media-to-doc-1.4.0-setup.exe"
InstallDir "$PROGRAMFILES\MediaToDoc"
InstallDirRegKey HKLM "${PRODUCT_DIR_REGKEY}" ""
RequestExecutionLevel admin

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"

Section "Install"
  SetOutPath "$INSTDIR"
  File "..\target\release\media-to-doc-ui.exe"
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
