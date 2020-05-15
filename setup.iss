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
Source: "{#SourcePath}\target\release\jorup.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

