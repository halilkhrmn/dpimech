# Release smoke test: installs the Windows installer silently, checks that the service runs and
# answers and that the GUI starts, then uninstalls. Needs an elevated shell (CI runners are).
# Usage: pwsh tools/smoke-windows.ps1 target/installer/dpimech-setup-<version>.exe
param([Parameter(Mandatory)][string]$Installer)
$ErrorActionPreference = 'Stop'
function Fail([string]$message) { Write-Error "SMOKE FAIL: $message"; exit 1 }

Write-Host '== install'
$p = Start-Process $Installer -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-' -Wait -PassThru
if ($p.ExitCode -ne 0) { Fail "the installer exited with code $($p.ExitCode)" }
$app = Join-Path $env:ProgramFiles 'DPIMech'

Write-Host '== service'
$deadline = (Get-Date).AddSeconds(30)
do {
  $svc = Get-Service dpimech -ErrorAction SilentlyContinue
  if ($svc -and $svc.Status -eq 'Running') { break }
  Start-Sleep -Milliseconds 500
} while ((Get-Date) -lt $deadline)
if (-not $svc -or $svc.Status -ne 'Running') { Fail "the dpimech service is not running ($($svc.Status))" }
$reply = & "$PSScriptRoot/ipc.ps1" -Requests '{"type":"hello","client_version":"smoke"}' -ListenSeconds 1 -Filter '"reply"'
if (-not ($reply -match 'service_version')) { Fail "the service did not answer: $reply" }
Write-Host $reply

Write-Host '== window'
$gui = Start-Process (Join-Path $app 'dpimech.exe') -PassThru
Start-Sleep -Seconds 10
if ($gui.HasExited) { Fail "the GUI exited with code $($gui.ExitCode)" }
Stop-Process -Id $gui.Id -Force

Write-Host '== uninstall'
$p = Start-Process (Join-Path $app 'unins000.exe') -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait -PassThru
if ($p.ExitCode -ne 0) { Fail "the uninstaller exited with code $($p.ExitCode)" }
# The uninstaller hands the rest to a copy of itself; give it a moment to remove the service.
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Service dpimech -ErrorAction SilentlyContinue) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
if (Get-Service dpimech -ErrorAction SilentlyContinue) { Fail 'the service is still installed after uninstalling' }
Write-Host "SMOKE OK: $(Split-Path $Installer -Leaf)"
