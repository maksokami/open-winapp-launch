#ifndef AppVersion
#define AppVersion "0.1.0"
#endif

#define AppName "App Launcher"
#define AppExe "open-winapp-launcher.exe"

[Setup]
AppId={{39B9FA77-EFEB-4E0B-8758-D4B647A7B755}
AppName={#AppName}
AppVersion={#AppVersion}
DefaultDirName={userpf}\AppLauncher
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
MinVersion=10.0.22000
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
CloseApplications=yes
RestartApplications=no
OutputDir=..\dist
OutputBaseFilename=open-winapp-launcher-setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#AppExe}
LicenseFile=..\LICENSE

[Tasks]
Name: "startup"; Description: "Start App Launcher when I sign in to Windows"; GroupDescription: "Additional options:"
Name: "launchnow"; Description: "Launch App Launcher after installation"; GroupDescription: "Additional options:"; Flags: unchecked
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional options:"; Flags: unchecked

[Files]
Source: "..\target\x86_64-pc-windows-msvc\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD_PARTY.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\THIRD_PARTY_LICENSES.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\THIRD_PARTY_NOTICES\*"; DestDir: "{app}\THIRD_PARTY_NOTICES"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\examples\config.yaml"; DestDir: "{app}\examples"; Flags: ignoreversion

[Icons]
Name: "{group}\App Launcher"; Filename: "{app}\{#AppExe}"
Name: "{userdesktop}\App Launcher"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "AppLauncher"; ValueData: """{app}\{#AppExe}"" --background"; Tasks: startup; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch App Launcher"; Flags: nowait postinstall skipifsilent; Tasks: launchnow
