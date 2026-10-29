; Build with: ISCC /DAppVersion=x.y.z installer\veluno.iss  (scripts\package.ps1 does this)
#ifndef AppVersion
  #error Pass /DAppVersion=x.y.z
#endif

[Setup]
AppId={{6F1D2C8E-4B7A-4E55-9D3A-FE1A215B0C11}
AppName=Veluno
AppVersion={#AppVersion}
AppPublisher=Veluno Browser
AppPublisherURL=https://github.com/khentmoba/Veluno
AppSupportURL=https://github.com/khentmoba/Veluno/issues
AppUpdatesURL=https://github.com/khentmoba/Veluno/releases
VersionInfoVersion={#AppVersion}.0
VersionInfoProductName=Veluno
VersionInfoDescription=Veluno Setup
DefaultDirName={localappdata}\Programs\Veluno
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
CloseApplications=yes
WizardStyle=modern
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\veluno.exe
OutputDir=..\target\release
OutputBaseFilename=veluno-setup
Compression=lzma2
SolidCompression=yes

[Tasks]
Name: desktopicon; Description: "{cm:CreateDesktopIcon}"; Flags: checkedonce

[Files]
Source: "..\target\release\veluno.exe"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
Type: files; Name: "{autoprograms}\Felaris.lnk"
Type: files; Name: "{autodesktop}\Felaris.lnk"
Type: files; Name: "{app}\felaris.exe"
Type: files; Name: "{app}\felaris.exe.old"
Type: files; Name: "{app}\felaris.exe.new"

[Icons]
Name: "{autoprograms}\Veluno"; Filename: "{app}\veluno.exe"; Comment: "Veluno Browser"; AppUserModelID: "Veluno.Browser"
Name: "{autodesktop}\Veluno"; Filename: "{app}\veluno.exe"; Comment: "Veluno Browser"; AppUserModelID: "Veluno.Browser"; Tasks: desktopicon

; Makes Veluno selectable as a default browser in Windows Settings; Windows keeps the final choice with the user.
[Registry]
Root: HKCU; Subkey: "Software\Classes\VelunoHTML"; ValueType: string; ValueData: "Veluno HTML Document"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\VelunoHTML"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\VelunoHTML\DefaultIcon"; ValueType: string; ValueData: """{app}\veluno.exe"",0"
Root: HKCU; Subkey: "Software\Classes\VelunoHTML\shell\open\command"; ValueType: string; ValueData: """{app}\veluno.exe"" -- ""%1"""
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno"; ValueType: string; ValueData: "Veluno"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\DefaultIcon"; ValueType: string; ValueData: """{app}\veluno.exe"",0"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\shell\open\command"; ValueType: string; ValueData: """{app}\veluno.exe"""
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Veluno"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Browse the web with Veluno."
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\Capabilities"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: """{app}\veluno.exe"",0"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\Capabilities\URLAssociations"; ValueType: string; ValueName: "http"; ValueData: "VelunoHTML"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\Capabilities\URLAssociations"; ValueType: string; ValueName: "https"; ValueData: "VelunoHTML"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\Capabilities\FileAssociations"; ValueType: string; ValueName: ".htm"; ValueData: "VelunoHTML"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Veluno\Capabilities\FileAssociations"; ValueType: string; ValueName: ".html"; ValueData: "VelunoHTML"
Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Veluno"; ValueData: "Software\Clients\StartMenuInternet\Veluno\Capabilities"; Flags: uninsdeletevalue
; Versions before the installer registered their own Installed apps entry; drop it so Veluno is listed once.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\Veluno"; ValueType: none; Flags: deletekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\Felaris"; ValueType: none; Flags: deletekey
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris"; ValueType: none; Flags: deletekey
Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueName: "Felaris"; ValueType: none; Flags: deletevalue
; Existing Windows default-browser choices still reference the old ProgID.
Root: HKCU; Subkey: "Software\Classes\FelarisHTML"; ValueType: string; ValueData: "Veluno HTML Document"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\FelarisHTML"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\FelarisHTML\DefaultIcon"; ValueType: string; ValueData: """{app}\veluno.exe"",0"
Root: HKCU; Subkey: "Software\Classes\FelarisHTML\shell\open\command"; ValueType: string; ValueData: """{app}\veluno.exe"" -- ""%1"""

[Run]
Filename: "{app}\veluno.exe"; Description: "{cm:LaunchProgram,Veluno}"; Flags: nowait postinstall skipifsilent
; In-app updates run silently; reopen Veluno (--updated waits for the old instance to exit).
Filename: "{app}\veluno.exe"; Parameters: "--updated"; Flags: nowait; Check: RelaunchAfterSilentInstall

; Browser data lives in %LOCALAPPDATA%\Veluno and is deliberately kept on uninstall.
[UninstallDelete]
Type: files; Name: "{app}\veluno.exe.old"
Type: files; Name: "{app}\veluno.exe.new"

[Code]
function RelaunchAfterSilentInstall: Boolean;
var
  I: Integer;
begin
  Result := WizardSilent;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/NOLAUNCH') = 0 then Result := False;
end;
