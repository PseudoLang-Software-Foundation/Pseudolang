!include "MUI2.nsh"
!include "nsDialogs.nsh"
!include "WinMessages.nsh"
!include "WordFunc.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif

RequestExecutionLevel user

Var IsAdminInstall
Var InstModeAllUsers
Var RadioUser
Var RadioAdmin

# Branding
!define MUI_ICON "../../assets/Pseudolang-Logo.ico"
!define MUI_UNICON "../../assets/Pseudolang-Logo.ico"

Name "PseudoLang v${VERSION}"
OutFile "../../dist/release/pseudolang-setup-amd64.exe"
BrandingText "(c) 2026 PseudoLang Software Foundation"

# EXE version metadata
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "PseudoLang"
VIAddVersionKey "CompanyName" "PseudoLang Software Foundation"
VIAddVersionKey "LegalCopyright" "(c) 2026 PseudoLang Software Foundation"
VIAddVersionKey "FileDescription" "PseudoLang Interpreter Installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"

# Pages
!insertmacro MUI_PAGE_WELCOME
Page custom InstModePageCreate InstModePageLeave
!insertmacro MUI_PAGE_LICENSE "../../LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Function .onInit
    StrCpy $IsAdminInstall 0
    StrCpy $InstModeAllUsers 0
    StrCpy $INSTDIR "$LOCALAPPDATA\PseudoLang"

    ${GetParameters} $0
    ${GetOptions} $0 "/allusers" $1
    ${IfNot} ${Errors}
        UserInfo::GetAccountType
        Pop $0
        ${If} $0 == "Admin"
            StrCpy $IsAdminInstall 1
            StrCpy $InstModeAllUsers 1
            StrCpy $INSTDIR "$PROGRAMFILES\PseudoLang"
        ${EndIf}
    ${EndIf}
FunctionEnd

Function InstModePageCreate
    ${If} $IsAdminInstall == 1
        Abort
    ${EndIf}

    nsDialogs::Create 1018
    Pop $0

    ${NSD_CreateLabel} 0 0 100% 24u "Choose how to install PseudoLang:"
    Pop $0

    ${NSD_CreateRadioButton} 12u 30u 100% 12u "Install just for me (recommended)"
    Pop $RadioUser
    ${NSD_Check} $RadioUser

    ${NSD_CreateRadioButton} 12u 48u 100% 12u "Install for all users (requires admin)"
    Pop $RadioAdmin

    nsDialogs::Show
FunctionEnd

Function InstModePageLeave
    ${NSD_GetState} $RadioAdmin $0
    ${If} $0 == ${BST_CHECKED}
        UserInfo::GetAccountType
        Pop $0
        ${If} $0 == "Admin"
            StrCpy $IsAdminInstall 1
            StrCpy $INSTDIR "$PROGRAMFILES\PseudoLang"
        ${Else}
            ExecShell "runas" "$EXEPATH" "/allusers" SW_SHOWNORMAL
            Quit
        ${EndIf}
    ${Else}
        StrCpy $IsAdminInstall 0
        StrCpy $INSTDIR "$LOCALAPPDATA\PseudoLang"
    ${EndIf}
FunctionEnd

Section "Install"
    SetOutPath $INSTDIR

    File /oname=fpli.exe "../../dist/release/fpli-amd64.exe"
    File "../../LICENSE"
    File "../../assets/Pseudolang-Logo.ico"

    ${If} $IsAdminInstall == 1
        # System PATH
        ReadRegStr $R0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
        WriteRegStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" "$INSTDIR;$R0"

        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayName" "Pseudolang"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayVersion" "${VERSION}"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "Publisher" "Pseudolang Software Foundation"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayIcon" "$INSTDIR\Pseudolang-Logo.ico"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "NoModify" 1
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "NoRepair" 1
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "UninstallString" "$INSTDIR\uninstall.exe"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "InstallMode" "Admin"
    ${Else}
        # User PATH
        ReadRegStr $R0 HKCU "Environment" "Path"
        WriteRegStr HKCU "Environment" "Path" "$INSTDIR;$R0"

        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayName" "Pseudolang"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayVersion" "${VERSION}"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "Publisher" "Pseudolang Software Foundation"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "DisplayIcon" "$INSTDIR\Pseudolang-Logo.ico"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "NoModify" 1
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "NoRepair" 1
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "UninstallString" "$INSTDIR\uninstall.exe"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "InstallMode" "User"
    ${EndIf}

    System::Call 'Kernel32::SetEnvironmentVariableA(t "PATH", t "$INSTDIR;$%PATH%")'
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000

    WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
    ReadRegStr $R2 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang" "InstallMode"
    ${If} $R2 == "Admin"
        ReadRegStr $R0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
        ${WordReplace} "$R0" "$INSTDIR;" "" "+" $R1
        WriteRegStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" "$R1"
        DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang"
    ${Else}
        ReadRegStr $R0 HKCU "Environment" "Path"
        ${WordReplace} "$R0" "$INSTDIR;" "" "+" $R1
        WriteRegStr HKCU "Environment" "Path" "$R1"
        DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Pseudolang"
    ${EndIf}

    Delete "$INSTDIR\fpli.exe"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\Pseudolang-Logo.ico"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
SectionEnd
