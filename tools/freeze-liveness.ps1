# Operator-started Windows observer. No service/firewall changes or SSK launch.
[CmdletBinding()]
param(
    [System.Net.IPAddress]$Target = '192.168.50.155',
    [ValidateRange(1, 900)][int]$Seconds = 120,
    [string]$OutputDir = (Join-Path $PSScriptRoot '..\runtime\freeze-liveness')
)
$ErrorActionPreference = 'Stop'
$SampleMillis = 1000
$TimeoutMillis = 500
$MaxErrorChars = 500
$OutputDir = [IO.Path]::GetFullPath($OutputDir)
[IO.Directory]::CreateDirectory($OutputDir) | Out-Null
$RunId = [guid]::NewGuid().ToString('N')
$Path = Join-Path $OutputDir ("ping-{0}-{1}.ndjson" -f [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ'), $RunId)
$Stream = [IO.FileStream]::new($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write,
    [IO.FileShare]::Read, 4096, [IO.FileOptions]::WriteThrough)
$Writer = [IO.StreamWriter]::new($Stream, [Text.UTF8Encoding]::new($false))
$Ping = [Net.NetworkInformation.Ping]::new()
$Clock = [Diagnostics.Stopwatch]::StartNew()
$Seq = 0
$Replies = 0
Write-Host "Windows liveness capture: $Path"
Write-Host 'Confirm replies BEFORE SSK. Missing ICMP alone does not prove kernel lockup (VPN/firewall/routing).'
try {
    while ($Clock.Elapsed.TotalSeconds -lt $Seconds) {
        $StartedMillis = $Clock.ElapsedMilliseconds
        $Row = [ordered]@{
            schema = 'ssk.liveness/1'; run_id = $RunId; seq = $Seq
            ts_utc = [DateTime]::UtcNow.ToString('o'); monotonic_ms = $StartedMillis
            target = $Target.ToString(); status = 'exception'; rtt_ms = $null
        }
        try {
            $Reply = $Ping.Send($Target, $TimeoutMillis)
            $Row.status = $Reply.Status.ToString()
            if ($Reply.Status -eq [Net.NetworkInformation.IPStatus]::Success) {
                $Row.rtt_ms = $Reply.RoundtripTime
                $Replies++
            }
        } catch {
            $Message = $_.Exception.Message
            $Row.error = $Message.Substring(0, [Math]::Min($Message.Length, $MaxErrorChars))
        }
        $Writer.WriteLine(($Row | ConvertTo-Json -Compress))
        $Writer.Flush()
        $Stream.Flush($true)
        $Seq++
        $WaitMillis = [Math]::Min($SampleMillis - ($Clock.ElapsedMilliseconds - $StartedMillis),
            $Seconds * $SampleMillis - $Clock.ElapsedMilliseconds)
        if ($WaitMillis -gt 0) { Start-Sleep -Milliseconds ([int]$WaitMillis) }
    }
} finally {
    $Ping.Dispose()
    $Writer.Dispose()
    Write-Host "Capture finished: $Seq samples, $Replies replies. SSK was not started."
}
