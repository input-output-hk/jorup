#define MyAppName "jorup"
#define MyAppVersion "0.4.0"
#define MyAppPublisher "Input Output HK Limited"
#define MyAppURL "https://input-output-hk.github.io/jorup/"
#define MyAppExeName "jorup.exe"

[Setup]
AppId={{6959DB51-B864-4DAE-BCA0-56F630E2C79D}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
AppReadmeFile={#SourcePath}\README.md
DefaultDirName={%USERPROFILE}\.jorup\bin
DisableDirPage=yes
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile={#SourcePath}\LICENSE-MIT
PrivilegesRequired=lowest
OutputDir={#SourcePath}\target\innosetup
OutputBaseFilename=jorup-init
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes
UsePreviousAppDir=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "{#SourcePath}\target\x86_64-pc-windows-msvc\release\jorup.exe"; \
  DestDir: "{app}"; \
  Flags: ignoreversion

[Registry]
Root: HKCU; \
  Subkey: "Environment"; \
  ValueType: expandsz; \
  ValueName: "Path"; \
  ValueData: "{app};{olddata}"; \
  Check: PathNeedUpdate()

[Icons]
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

[UninstallDelete]
Type: filesandordirs; Name: "{app}/../release"
Type: filesandordirs; Name: "{app}/../jorfile.json"

[Code]

function PathNeedUpdate: boolean;
var
  needle, haystack: String;
begin
  needle := ExpandConstant('{app};');
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', haystack)
  then
    Result := Pos(needle, haystack) = 0
  else
    Result := True;
end;

procedure PathUninstall;
var
  needle, path: String;
begin
  needle := ExpandConstant('{app};');
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', path)
  then
    if StringChangeEx(path, needle, '', False) > 0
    then
      RegWriteStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', path);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall
  then
    PathUninstall;
  if CurUninstallStep = usDone
  then
    MsgBox(
      ExpandConstant('Your data (keys, configuration files) still remains in {app}. Take care of it.'), 
      mbInformation,
      MB_OK);
end;
