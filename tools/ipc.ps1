param([string[]]$Requests, [int]$ListenSeconds = 3, [string]$Filter = '', [string]$Pipe = $(if ($env:DPIMECH_PIPE) { $env:DPIMECH_PIPE } else { "dpimech" }))
# Sends each JSON request (without id) to the dpimech pipe and prints every frame received.
$p = New-Object System.IO.Pipes.NamedPipeClientStream(".", $Pipe, [System.IO.Pipes.PipeDirection]::InOut, [System.IO.Pipes.PipeOptions]::Asynchronous)
$p.Connect(3000)
$w = New-Object System.IO.StreamWriter($p); $r = New-Object System.IO.StreamReader($p)
$id = 0
$pending = $r.ReadLineAsync()
function Drain([int]$ms) {
  $deadline = [DateTime]::Now.AddMilliseconds($ms)
  while ([DateTime]::Now -lt $deadline) {
    if ($script:pending.Wait(100)) {
      $line = $script:pending.Result
      if ($null -eq $line) { return }
      if (-not $Filter -or $line -match $Filter) { $line }
      $script:pending = $r.ReadLineAsync()
    }
  }
}
foreach ($req in $Requests) {
  $id++
  $w.WriteLine("{`"id`":$id,`"request`":$req}"); $w.Flush()
  Drain 400
}
Drain ($ListenSeconds * 1000)
$p.Dispose()
