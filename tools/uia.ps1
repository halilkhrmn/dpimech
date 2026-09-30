param([string]$List, [string]$Invoke, [string]$Toggle)
# Drives the dpimech window through Windows UI Automation (Slint exposes it via AccessKit).
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$proc = Get-Process dpimech -ErrorAction Stop | Select-Object -First 1
$root = [System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)
$all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
if ($List) {
  foreach ($e in $all) {
    $n = $e.Current.Name; $t = $e.Current.ControlType.ProgrammaticName
    if ($n -and ($List -eq '*' -or $n -match $List)) { "{0,-28} {1}" -f $t, $n }
  }
}
function Find([string]$name, [string]$type) {
  foreach ($e in $all) { if ($e.Current.Name -eq $name -and $e.Current.ControlType.ProgrammaticName -match $type) { return $e } }
}
if ($Invoke) {
  $e = Find $Invoke 'Button'
  if (-not $e) { "not found: $Invoke"; exit 1 }
  $e.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke(); "invoked $Invoke"
}
if ($Toggle) {
  # Toggles the switch whose accessible label equals $Toggle.
  foreach ($e in $all) {
    $pat = $null
    if ($e.Current.Name -eq $Toggle -and $e.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pat)) {
      $pat.Toggle(); "toggled $Toggle"; exit 0
    }
  }
  "no switch named $Toggle"; exit 1
}
