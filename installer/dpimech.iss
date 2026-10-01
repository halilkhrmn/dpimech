; DPIMech installer (Inno Setup 6). Build with tools\build-installer.ps1.
; Installs the GUI and the background service into Program Files, registers the service
; (which secures C:\ProgramData\dpimech), and adds a Start menu shortcut.

#ifndef AppVersion
  #define AppVersion "0.2.7"
#endif
#define AppName "DPIMech"
; 0.1.x shipped under the working name "dpimngr"; upgrades clean it up (see below).
#define LegacyName "dpimngr"
#define AppExe "dpimech.exe"
#define ServiceExe "dpimech-service.exe"
#define AppUserModelID "dpimech.app"

[Setup]
AppId={{6B0F5B2E-3C1D-4E8A-9A57-2F0D8C1B7E41}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=DPIMech contributors
AppPublisherURL=https://github.com/halilkhrmn/dpimech
DefaultDirName={autopf}\{#AppName}
; Same AppId as 0.1.x so Windows lists one app, but never reuse the old "dpimngr" folder.
UsePreviousAppDir=no
DisableDirPage=yes
DisableProgramGroupPage=yes
; The service runs as SYSTEM, so its folder must stay admin-only (Program Files).
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\target\installer
OutputBaseFilename=dpimech-setup-{#AppVersion}
SetupIconFile=..\crates\gui\assets\dpimech.ico
UninstallDisplayIcon={app}\{#AppExe}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\{#ServiceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[InstallDelete]
; Leftovers of 0.1.x. PrepareToInstall stops the old service and GUI first so nothing is
; locked; `dpimech-service install` then removes the old service registration and moves
; C:\ProgramData\dpimngr (profiles, engines) to C:\ProgramData\dpimech.
Type: filesandordirs; Name: "{autopf}\{#LegacyName}"
Type: files; Name: "{autoprograms}\{#LegacyName}.lnk"
Type: files; Name: "{autodesktop}\{#LegacyName}.lnk"

[Icons]
; The AppUserModelID ties toasts and the taskbar to this shortcut's name and icon.
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"; AppUserModelID: "{#AppUserModelID}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon; AppUserModelID: "{#AppUserModelID}"

[Run]
Filename: "{app}\{#ServiceExe}"; Parameters: "install"; StatusMsg: "Installing the background service..."; Flags: runhidden waituntilterminated
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent runasoriginaluser

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/IM {#AppExe} /F"; Flags: runhidden; RunOnceId: "StopGui"
Filename: "{app}\{#ServiceExe}"; Parameters: "uninstall"; Flags: runhidden waituntilterminated; RunOnceId: "RemoveService"

[Code]
// Upgrades: the running service and GUI lock their files, so stop both before copying.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM {#AppExe} /F', '', SW_HIDE, ewWaitUntilTerminated, Code);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM {#LegacyName}.exe /F', '', SW_HIDE, ewWaitUntilTerminated, Code);
  if Exec(ExpandConstant('{sys}\sc.exe'), 'stop dpimech', '', SW_HIDE, ewWaitUntilTerminated, Code) then
    Sleep(2000);
  if Exec(ExpandConstant('{sys}\sc.exe'), 'stop {#LegacyName}', '', SW_HIDE, ewWaitUntilTerminated, Code) then
    Sleep(2000);
  Result := '';
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Code: Integer;
begin
  // "Start DPIMech when I sign in" lives in the user's Run key; remove it with the app.
  if CurUninstallStep = usPostUninstall then
    Exec(ExpandConstant('{sys}\reg.exe'),
      'delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v dpimech /f',
      '', SW_HIDE, ewWaitUntilTerminated, Code);
end;
