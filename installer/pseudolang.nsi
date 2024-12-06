!include "MUI2.nsh"
!include "WinMessages.nsh"
!include "WordFunc.nsh"

!define MUI_ICON "Pseudolang-Logo.ico"

Name "PseudoLang Installer v0.9.511"
InstallDir "$PROGRAMFILES\PseudoLang\"
OutFile "../release/installer/pseudolang-setup-x64.exe"
BrandingText "(c) 2024 PseudoLang Software Foundation"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_DIRECTORY
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Section ""
    SetOutPath $INSTDIR

    File /oname=fplc.exe "fplc.exe"
    
    System::Call 'Kernel32::SetEnvironmentVariableA(t "PATH", t "$INSTDIR;$%PATH%")'

    ReadRegStr $R0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
    WriteRegStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" "$INSTDIR;$R0"
    
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayName" "Pseudolang"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayVersion" "0.9.511"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "Publisher" "Pseudolang Software Foundation"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayIcon" "$INSTDIR\Pseudolang-Logo.ico"

    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "NoModify" 1
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "NoRepair" 1
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "UninstallString" "$INSTDIR\uninstall.exe"

    File "LICENSE"
    File "Pseudolang-Logo.ico"
    
    WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
    ReadRegStr $R0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
    ${WordReplace} "$R0" "$INSTDIR;" "" "+" $R1
    WriteRegStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" "$R1"

    Delete "$INSTDIR\fplc.exe"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\Pseudolang-Logo.ico"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang"
    
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
SectionEnd