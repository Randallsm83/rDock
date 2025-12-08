# Dock Performance Profiler
# Compares Nexus Dock and rdock performance metrics

param(
    [int]$DurationSeconds = 30,
    [int]$SampleIntervalMs = 500
)

$NexusPath = "C:\Program Files (x86)\Winstep\Nexus.exe"
$RdockPath = "C:\Users\Randall\projects\rdock\target\release\rdock.exe"

Write-Host "`n=== Dock Performance Profiler ===" -ForegroundColor Cyan
Write-Host "Duration: $DurationSeconds seconds" -ForegroundColor Gray
Write-Host "Sample Interval: $SampleIntervalMs ms`n" -ForegroundColor Gray

function Get-ProcessMetrics {
    param([string]$ProcessName)
    
    $proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
    if (-not $proc) {
        return $null
    }
    
    return @{
        CPU = $proc.CPU
        WorkingSet = $proc.WorkingSet64
        PrivateMemory = $proc.PrivateMemorySize64
        Threads = $proc.Threads.Count
        Handles = $proc.HandleCount
    }
}

function Profile-Dock {
    param(
        [string]$Name,
        [string]$ProcessName,
        [string]$ExePath
    )
    
    Write-Host "Profiling $Name..." -ForegroundColor Yellow
    
    $proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
    $wasRunning = $null -ne $proc
    
    if (-not $wasRunning) {
        Write-Host "  Starting $Name..." -ForegroundColor Gray
        Start-Process $ExePath
        Start-Sleep -Seconds 3  # Let it initialize
    }
    
    $samples = @()
    $sampleCount = [Math]::Ceiling($DurationSeconds * 1000 / $SampleIntervalMs)
    
    Write-Host "  Collecting $sampleCount samples..." -ForegroundColor Gray
    
    for ($i = 0; $i -lt $sampleCount; $i++) {
        $metrics = Get-ProcessMetrics -ProcessName $ProcessName
        if ($metrics) {
            $samples += $metrics
        }
        Start-Sleep -Milliseconds $SampleIntervalMs
        
        if (($i + 1) % 10 -eq 0) {
            Write-Host "    Sample $($i + 1)/$sampleCount" -ForegroundColor DarkGray
        }
    }
    
    if (-not $wasRunning) {
        Write-Host "  Stopping $Name..." -ForegroundColor Gray
        Stop-Process -Name $ProcessName -Force -ErrorAction SilentlyContinue
    }
    
    return $samples
}

function Get-Statistics {
    param([array]$Samples, [string]$Property)
    
    $values = $Samples | ForEach-Object { $_.$Property }
    $sorted = $values | Sort-Object
    
    return @{
        Min = $sorted[0]
        Max = $sorted[-1]
        Avg = ($values | Measure-Object -Average).Average
        Median = if ($sorted.Count % 2 -eq 0) {
            ($sorted[$sorted.Count/2 - 1] + $sorted[$sorted.Count/2]) / 2
        } else {
            $sorted[[Math]::Floor($sorted.Count/2)]
        }
    }
}

function Format-Bytes {
    param([long]$Bytes)
    
    if ($Bytes -ge 1GB) {
        return "{0:N2} GB" -f ($Bytes / 1GB)
    } elseif ($Bytes -ge 1MB) {
        return "{0:N2} MB" -f ($Bytes / 1MB)
    } elseif ($Bytes -ge 1KB) {
        return "{0:N2} KB" -f ($Bytes / 1KB)
    } else {
        return "{0} bytes" -f $Bytes
    }
}

function Display-Results {
    param(
        [string]$Name,
        [array]$Samples
    )
    
    Write-Host "`n=== $Name Results ===" -ForegroundColor Cyan
    
    # CPU Time (total accumulated)
    $cpuStats = Get-Statistics -Samples $Samples -Property "CPU"
    Write-Host "`nCPU Time (accumulated):" -ForegroundColor Green
    Write-Host ("  Min:    {0:N2}s" -f $cpuStats.Min)
    Write-Host ("  Max:    {0:N2}s" -f $cpuStats.Max)
    Write-Host ("  Avg:    {0:N2}s" -f $cpuStats.Avg)
    Write-Host ("  Median: {0:N2}s" -f $cpuStats.Median)
    
    # Memory - Working Set
    $memStats = Get-Statistics -Samples $Samples -Property "WorkingSet"
    Write-Host "`nMemory (Working Set):" -ForegroundColor Green
    Write-Host ("  Min:    " + (Format-Bytes $memStats.Min))
    Write-Host ("  Max:    " + (Format-Bytes $memStats.Max))
    Write-Host ("  Avg:    " + (Format-Bytes $memStats.Avg))
    Write-Host ("  Median: " + (Format-Bytes $memStats.Median))
    
    # Memory - Private
    $privStats = Get-Statistics -Samples $Samples -Property "PrivateMemory"
    Write-Host "`nMemory (Private):" -ForegroundColor Green
    Write-Host ("  Min:    " + (Format-Bytes $privStats.Min))
    Write-Host ("  Max:    " + (Format-Bytes $privStats.Max))
    Write-Host ("  Avg:    " + (Format-Bytes $privStats.Avg))
    Write-Host ("  Median: " + (Format-Bytes $privStats.Median))
    
    # Threads
    $threadStats = Get-Statistics -Samples $Samples -Property "Threads"
    Write-Host "`nThreads:" -ForegroundColor Green
    Write-Host ("  Min:    {0}" -f $threadStats.Min)
    Write-Host ("  Max:    {0}" -f $threadStats.Max)
    Write-Host ("  Avg:    {0:N1}" -f $threadStats.Avg)
    Write-Host ("  Median: {0}" -f $threadStats.Median)
    
    # Handles
    $handleStats = Get-Statistics -Samples $Samples -Property "Handles"
    Write-Host "`nHandles:" -ForegroundColor Green
    Write-Host ("  Min:    {0}" -f $handleStats.Min)
    Write-Host ("  Max:    {0}" -f $handleStats.Max)
    Write-Host ("  Avg:    {0:N1}" -f $handleStats.Avg)
    Write-Host ("  Median: {0}" -f $handleStats.Median)
}

function Compare-Results {
    param(
        [array]$NexusSamples,
        [array]$RdockSamples
    )
    
    Write-Host "`n`n=== Comparison ===" -ForegroundColor Cyan
    
    $nexusMem = (Get-Statistics -Samples $NexusSamples -Property "WorkingSet").Avg
    $rdockMem = (Get-Statistics -Samples $RdockSamples -Property "WorkingSet").Avg
    $memDiff = (($nexusMem - $rdockMem) / $nexusMem) * 100
    
    $nexusPriv = (Get-Statistics -Samples $NexusSamples -Property "PrivateMemory").Avg
    $rdockPriv = (Get-Statistics -Samples $RdockSamples -Property "PrivateMemory").Avg
    $privDiff = (($nexusPriv - $rdockPriv) / $nexusPriv) * 100
    
    $nexusThreads = (Get-Statistics -Samples $NexusSamples -Property "Threads").Avg
    $rdockThreads = (Get-Statistics -Samples $RdockSamples -Property "Threads").Avg
    
    $nexusHandles = (Get-Statistics -Samples $NexusSamples -Property "Handles").Avg
    $rdockHandles = (Get-Statistics -Samples $RdockSamples -Property "Handles").Avg
    
    Write-Host "`nMemory (Working Set):" -ForegroundColor Green
    Write-Host ("  Nexus:  " + (Format-Bytes $nexusMem))
    Write-Host ("  rdock:  " + (Format-Bytes $rdockMem))
    if ($memDiff -gt 0) {
        Write-Host ("  rdock uses {0:N1}% less memory" -f $memDiff) -ForegroundColor Yellow
    } else {
        Write-Host ("  rdock uses {0:N1}% more memory" -f ([Math]::Abs($memDiff))) -ForegroundColor Red
    }
    
    Write-Host "`nMemory (Private):" -ForegroundColor Green
    Write-Host ("  Nexus:  " + (Format-Bytes $nexusPriv))
    Write-Host ("  rdock:  " + (Format-Bytes $rdockPriv))
    if ($privDiff -gt 0) {
        Write-Host ("  rdock uses {0:N1}% less private memory" -f $privDiff) -ForegroundColor Yellow
    } else {
        Write-Host ("  rdock uses {0:N1}% more private memory" -f ([Math]::Abs($privDiff))) -ForegroundColor Red
    }
    
    Write-Host "`nThreads (Avg):" -ForegroundColor Green
    Write-Host ("  Nexus:  {0:N1}" -f $nexusThreads)
    Write-Host ("  rdock:  {0:N1}" -f $rdockThreads)
    
    Write-Host "`nHandles (Avg):" -ForegroundColor Green
    Write-Host ("  Nexus:  {0:N1}" -f $nexusHandles)
    Write-Host ("  rdock:  {0:N1}" -f $rdockHandles)
    
    # File sizes
    $nexusSize = (Get-Item $NexusPath).Length
    $rdockSize = (Get-Item $RdockPath).Length
    
    Write-Host "`nBinary Size:" -ForegroundColor Green
    Write-Host ("  Nexus:  " + (Format-Bytes $nexusSize))
    Write-Host ("  rdock:  " + (Format-Bytes $rdockSize))
    $sizeDiff = (($nexusSize - $rdockSize) / $nexusSize) * 100
    if ($sizeDiff -gt 0) {
        Write-Host ("  rdock is {0:N1}% smaller" -f $sizeDiff) -ForegroundColor Yellow
    } else {
        Write-Host ("  rdock is {0:N1}% larger" -f ([Math]::Abs($sizeDiff))) -ForegroundColor Red
    }
}

# Main execution
$nexusSamples = Profile-Dock -Name "Nexus Dock" -ProcessName "Nexus" -ExePath $NexusPath
$rdockSamples = Profile-Dock -Name "rdock" -ProcessName "rdock" -ExePath $RdockPath

Display-Results -Name "Nexus Dock" -Samples $nexusSamples
Display-Results -Name "rdock" -Samples $rdockSamples
Compare-Results -NexusSamples $nexusSamples -RdockSamples $rdockSamples

Write-Host "`n`nProfile complete!`n" -ForegroundColor Cyan
