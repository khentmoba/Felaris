; Build with: ISCC /DAppVersion=x.y.z installer\felaris.iss  (scripts\package.ps1 does this)
#ifndef AppVersion
  #error Pass /DAppVersion=x.y.z
#endif

[Setup]
AppId={{6F1D2C8E-4B7A-4E55-9D3A-FE1A215B0C11}
AppName=Felaris
AppVersion={#AppVersion}
AppPublisher=Felaris Browser
AppPublisherURL=https://github.com/khentmoba/Felaris
AppSupportURL=https://github.com/khentmoba/Felaris/issues
AppUpdatesURL=https://github.com/khentmoba/Felaris/releases
VersionInfoVersion={#AppVersion}.0
VersionInfoProductName=Felaris
VersionInfoDescription=Felaris Setup
DefaultDirName={localappdata}\Programs\Felaris
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
CloseApplications=yes
WizardStyle=modern
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\felaris.exe
OutputDir=..\target\release
OutputBaseFilename=felaris-setup
Compression=lzma2
SolidCompression=yes

[Tasks]
Name: desktopicon; Description: "{cm:CreateDesktopIcon}"; Flags: checkedonce

[Files]
Source: "..\target\release\felaris.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Felaris"; Filename: "{app}\felaris.exe"; Comment: "Felaris Browser"; AppUserModelID: "Felaris.Browser"
Name: "{autodesktop}\Felaris"; Filename: "{app}\felaris.exe"; Comment: "Felaris Browser"; AppUserModelID: "Felaris.Browser"; Tasks: desktopicon

; Makes Felaris selectable as a default browser in Windows Settings; Windows keeps the final choice with the user.
[Registry]
Root: HKCU; Subkey: "Software\Classes\FelarisHTML"; ValueType: string; ValueData: "Felaris HTML Document"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\FelarisHTML"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\FelarisHTML\DefaultIcon"; ValueType: string; ValueData: """{app}\felaris.exe"",0"
Root: HKCU; Subkey: "Software\Classes\FelarisHTML\shell\open\command"; ValueType: string; ValueData: """{app}\felaris.exe"" -- ""%1"""
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris"; ValueType: string; ValueData: "Felaris"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\DefaultIcon"; ValueType: string; ValueData: """{app}\felaris.exe"",0"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\shell\open\command"; ValueType: string; ValueData: """{app}\felaris.exe"""
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Felaris"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Browse the web with Felaris."
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\Capabilities"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: """{app}\felaris.exe"",0"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\Capabilities\URLAssociations"; ValueType: string; ValueName: "http"; ValueData: "FelarisHTML"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\Capabilities\URLAssociations"; ValueType: string; ValueName: "https"; ValueData: "FelarisHTML"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\Capabilities\FileAssociations"; ValueType: string; ValueName: ".htm"; ValueData: "FelarisHTML"
Root: HKCU; Subkey: "Software\Clients\StartMenuInternet\Felaris\Capabilities\FileAssociations"; ValueType: string; ValueName: ".html"; ValueData: "FelarisHTML"
Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Felaris"; ValueData: "Software\Clients\StartMenuInternet\Felaris\Capabilities"; Flags: uninsdeletevalue
; Versions before the installer registered their own Installed apps entry; drop it so Felaris is listed once.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\Felaris"; ValueType: none; Flags: deletekey

[Run]
Filename: "{app}\felaris.exe"; Description: "{cm:LaunchProgram,Felaris}"; Flags: nowait postinstall skipifsilent
; In-app updates run silently; reopen Felaris (--updated waits for the old instance to exit).
Filename: "{app}\felaris.exe"; Parameters: "--updated"; Flags: nowait; Check: RelaunchAfterSilentInstall

; Browser data lives in %LOCALAPPDATA%\Felaris and is deliberately kept on uninstall.
[UninstallDelete]
Type: files; Name: "{app}\felaris.exe.old"
Type: files; Name: "{app}\felaris.exe.new"

[Code]
function RelaunchAfterSilentInstall: Boolean;
var
  I: Integer;
begin
  Result := WizardSilent;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/NOLAUNCH') = 0 then Result := False;
end;
