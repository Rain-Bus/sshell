#define MyAppName "sshell"
#define MyAppVersion GetEnv("SHELL_VERSION")
#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif
#define MyAppPublisher "rain-bus"
#define MyAppURL "https://github.com/Rain-Bus/sshell"
#define MyAppExeName "sshell.exe"

#if GetEnv("SSH_TARGET") == "aarch64-pc-windows-msvc"
  #define MyArch "arm64"
  #define MyArchAllowed "arm64"
  #define MyArchInstall64 "arm64"
#else
  #define MyArch "x64compatible"
  #define MyArchAllowed "x64compatible"
  #define MyArchInstall64 "x64compatible"
#endif

[Setup]
AppId={{sshell-2024-1}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputBaseFilename=sshell-{#MyAppVersion}-{#MyArch}-windows-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed={#MyArchAllowed}
ArchitecturesInstallIn64BitMode={#MyArchInstall64}
OutputDir=.

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "addtopath"; Description: "Add sshell to PATH"; GroupDescription: "Environment:"

[Files]
Source: "sshell.exe"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; \
    ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; \
    Flags: preservestringtype uninsdeletevalue; \
    Tasks: addtopath

[UninstallDelete]
Type: files; Name: "{app}\sshell.exe"

[Code]
const
  WM_SETTINGCHANGE = $001A;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    // Notify the system that environment variables have changed
    RegWriteStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
      'Path', RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path'));
    Exec('cmd.exe', '/C echo %Path%', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;
