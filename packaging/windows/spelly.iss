; Builds Spelly-Setup.exe. Run from the repo root:
;   iscc /DAppVersion=0.1.0 packaging\windows\spelly.iss
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6E1F2B52-5D0A-4C6B-9B1E-3C5C8E7A1D42}
AppName=Spelly
AppVersion={#AppVersion}
AppPublisher=Spelly
DefaultDirName={autopf}\Spelly
DefaultGroupName=Spelly
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist
OutputBaseFilename=Spelly-Setup
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\spelly.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; The running app holds a mutex; setup asks to close it instead of failing mid-copy.
AppMutex=Local\SpellyRunning
CloseApplications=yes

[Tasks]
Name: "autostart"; Description: "Start Spelly when I sign in to Windows"; GroupDescription: "Options:"

[Files]
Source: "..\..\target\release\spelly.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Spelly"; Filename: "{app}\spelly.exe"
Name: "{group}\Uninstall Spelly"; Filename: "{uninstallexe}"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Spelly"; ValueData: """{app}\spelly.exe"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\spelly.exe"; Description: "Start Spelly now"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM spelly.exe"; Flags: runhidden; RunOnceId: "StopSpelly"
