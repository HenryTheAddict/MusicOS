param(
    [int]$TrackCount = 32,
    [int]$Port = 18180,
    [int]$Samples = 20,
    [int]$TimeoutSeconds = 45,
    [string]$Resolution = "640x480",
    [double]$MaxFirstScreenshotMs = 1500,
    [double]$MaxFrameP95Ms = 25,
    [double]$MaxApiP95Ms = 30,
    [switch]$SkipBuild,
    [switch]$KeepRunning
)

$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

$OutDir = Join-Path $Root "out\perf"
$MusicDir = Join-Path $OutDir "music-smoke"
$DbPath = Join-Path $OutDir "perf-smoke.sqlite3"
$ReportPath = Join-Path $OutDir "runtime-performance-smoke.json"
$StdoutPath = Join-Path $OutDir "musicos-perf.out.log"
$StderrPath = Join-Path $OutDir "musicos-perf.err.log"
$BaseUrl = "http://127.0.0.1:$Port"

function New-WavBytes {
    $sampleRate = 8000
    $seconds = 1
    $channels = 1
    $bitsPerSample = 16
    $blockAlign = [int]($channels * $bitsPerSample / 8)
    $byteRate = $sampleRate * $blockAlign
    $dataLength = $sampleRate * $seconds * $blockAlign
    $bytes = New-Object byte[] (44 + $dataLength)

    function Write-Ascii([byte[]]$Buffer, [int]$Offset, [string]$Text) {
        $ascii = [Text.Encoding]::ASCII.GetBytes($Text)
        [Array]::Copy($ascii, 0, $Buffer, $Offset, $ascii.Length)
    }
    function Write-U16([byte[]]$Buffer, [int]$Offset, [uint16]$Value) {
        $Buffer[$Offset] = [byte]($Value -band 0xff)
        $Buffer[$Offset + 1] = [byte](($Value -shr 8) -band 0xff)
    }
    function Write-U32([byte[]]$Buffer, [int]$Offset, [uint32]$Value) {
        $Buffer[$Offset] = [byte]($Value -band 0xff)
        $Buffer[$Offset + 1] = [byte](($Value -shr 8) -band 0xff)
        $Buffer[$Offset + 2] = [byte](($Value -shr 16) -band 0xff)
        $Buffer[$Offset + 3] = [byte](($Value -shr 24) -band 0xff)
    }

    Write-Ascii $bytes 0 "RIFF"
    Write-U32 $bytes 4 ([uint32](36 + $dataLength))
    Write-Ascii $bytes 8 "WAVE"
    Write-Ascii $bytes 12 "fmt "
    Write-U32 $bytes 16 16
    Write-U16 $bytes 20 1
    Write-U16 $bytes 22 ([uint16]$channels)
    Write-U32 $bytes 24 ([uint32]$sampleRate)
    Write-U32 $bytes 28 ([uint32]$byteRate)
    Write-U16 $bytes 32 ([uint16]$blockAlign)
    Write-U16 $bytes 34 ([uint16]$bitsPerSample)
    Write-Ascii $bytes 36 "data"
    Write-U32 $bytes 40 ([uint32]$dataLength)
    $bytes
}

function Wait-ForResult([string]$Name, [scriptblock]$Probe) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        try {
            $value = & $Probe
            if ($value) {
                return $value
            }
        } catch {
        }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)

    throw "Timed out waiting for $Name"
}

function Measure-Get([string]$Path, [int]$Count) {
    $times = New-Object System.Collections.Generic.List[double]
    $errors = @{}
    $bytes = 0
    for ($i = 0; $i -lt $Count; $i++) {
        $sw = [Diagnostics.Stopwatch]::StartNew()
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl$Path"
            $bytes = [int]$response.RawContentLength
        } catch {
            $key = $_.Exception.Message
            if ($_.Exception.Response) {
                $key = [string][int]$_.Exception.Response.StatusCode
            }
            $errors[$key] = 1 + ($errors[$key] | ForEach-Object { if ($_ -eq $null) { 0 } else { $_ } })
        } finally {
            $sw.Stop()
            $times.Add($sw.Elapsed.TotalMilliseconds)
        }
        Start-Sleep -Milliseconds 25
    }
    $sorted = @($times | Sort-Object)
    $p95Index = [Math]::Min($sorted.Count - 1, [Math]::Max(0, [int][Math]::Ceiling($sorted.Count * 0.95) - 1))
    [pscustomobject]@{
        path = $Path
        count = $Count
        errors = $errors
        min_ms = [Math]::Round(($sorted | Select-Object -First 1), 2)
        avg_ms = [Math]::Round((($times | Measure-Object -Average).Average), 2)
        p95_ms = [Math]::Round($sorted[$p95Index], 2)
        max_ms = [Math]::Round(($sorted | Select-Object -Last 1), 2)
        bytes = $bytes
    }
}

function Measure-Input([int]$Count) {
    $times = New-Object System.Collections.Generic.List[double]
    for ($i = 0; $i -lt $Count; $i++) {
        $key = @("Down", "Up", "Right", "Left")[$i % 4]
        $sw = [Diagnostics.Stopwatch]::StartNew()
        Invoke-WebRequest -UseBasicParsing -Method POST -Uri "$BaseUrl/api/input?key=$key" | Out-Null
        $sw.Stop()
        $times.Add($sw.Elapsed.TotalMilliseconds)
        Start-Sleep -Milliseconds 25
    }
    [pscustomobject]@{
        count = $Count
        avg_ms = [Math]::Round((($times | Measure-Object -Average).Average), 2)
        max_ms = [Math]::Round((($times | Measure-Object -Maximum).Maximum), 2)
    }
}

function Measure-Frames([int]$Count) {
    $values = New-Object System.Collections.Generic.List[double]
    for ($i = 0; $i -lt $Count; $i++) {
        $diagnostics = Invoke-RestMethod -Uri "$BaseUrl/api/diagnostics"
        if ($diagnostics.frame_ms -ne $null) {
            $values.Add([double]$diagnostics.frame_ms)
        }
        Start-Sleep -Milliseconds 50
    }
    if ($values.Count -eq 0) {
        return [pscustomobject]@{
            count = 0
            avg_ms = $null
            p95_ms = $null
            max_ms = $null
        }
    }
    $sorted = @($values | Sort-Object)
    $p95Index = [Math]::Min($sorted.Count - 1, [Math]::Max(0, [int][Math]::Ceiling($sorted.Count * 0.95) - 1))
    [pscustomobject]@{
        count = $values.Count
        avg_ms = [Math]::Round((($values | Measure-Object -Average).Average), 2)
        p95_ms = [Math]::Round($sorted[$p95Index], 2)
        max_ms = [Math]::Round((($values | Measure-Object -Maximum).Maximum), 2)
    }
}

function Assert-PerfThresholds($Report) {
    $failures = New-Object System.Collections.Generic.List[string]
    if ($Report.first_screenshot_ms -gt $MaxFirstScreenshotMs) {
        $failures.Add("first screenshot $($Report.first_screenshot_ms) ms > $MaxFirstScreenshotMs ms")
    }
    if ($Report.frame.p95_ms -gt $MaxFrameP95Ms) {
        $failures.Add("frame p95 $($Report.frame.p95_ms) ms > $MaxFrameP95Ms ms")
    }
    foreach ($entry in $Report.api) {
        if ($entry.path -in @("/api/status", "/api/library", "/api/diagnostics", "/api/player") -and $entry.p95_ms -gt $MaxApiP95Ms) {
            $failures.Add("$($entry.path) p95 $($entry.p95_ms) ms > $MaxApiP95Ms ms")
        }
        if ($entry.errors.Count -gt 0) {
            $failures.Add("$($entry.path) returned errors")
        }
    }
    if ($Report.first_screenshot_bytes -lt 1000) {
        $failures.Add("first screenshot was too small: $($Report.first_screenshot_bytes) bytes")
    }
    if ($Report.counts.tracks -lt $TrackCount) {
        $failures.Add("indexed $($Report.counts.tracks) tracks, expected at least $TrackCount")
    }
    if (-not ($Report.scan_status -like "Scan complete:*")) {
        $failures.Add("scan did not complete: $($Report.scan_status)")
    }

    if ($failures.Count -gt 0) {
        throw "Performance smoke failed: $($failures -join '; ')"
    }
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Remove-Item -Force -ErrorAction SilentlyContinue $DbPath, "$DbPath-shm", "$DbPath-wal", $ReportPath, $StdoutPath, $StderrPath
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $MusicDir
New-Item -ItemType Directory -Force -Path $MusicDir | Out-Null

$wavBytes = New-WavBytes
for ($i = 1; $i -le $TrackCount; $i++) {
    $artist = "Perf Artist {0:D2}" -f (($i % 4) + 1)
    $album = "Perf Album {0:D2}" -f (($i % 6) + 1)
    $albumDir = Join-Path $MusicDir (Join-Path $artist $album)
    New-Item -ItemType Directory -Force -Path $albumDir | Out-Null
    $trackPath = Join-Path $albumDir ("{0:D2} - Perf Track {0:D2}.wav" -f $i)
    [IO.File]::WriteAllBytes($trackPath, $wavBytes)
}

$Exe = Join-Path $Root "target\debug\musicos-mac.exe"
if (-not $SkipBuild) {
    cargo build --bin musicos-mac --features desktop-app,desktop-audio
} elseif (-not (Test-Path $Exe)) {
    throw "Missing $Exe. Run without -SkipBuild to build it first."
}

$oldEnv = @{}
foreach ($name in @("MUSICOS_COMPANION_PORT", "MUSICOS_MEDIA_DB", "MUSICOS_UPLOAD_DIR", "MUSICOS_SYNC_DIR", "MUSICOS_REINDEX_ALL", "MUSICOS_SETTINGS_PATH", "MUSICOS_RESOLUTION")) {
    $oldEnv[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

$process = $null
try {
    $env:MUSICOS_COMPANION_PORT = [string]$Port
    $env:MUSICOS_MEDIA_DB = $DbPath
    $env:MUSICOS_UPLOAD_DIR = $MusicDir
    $env:MUSICOS_SYNC_DIR = $MusicDir
    $env:MUSICOS_REINDEX_ALL = "1"
    $env:MUSICOS_SETTINGS_PATH = Join-Path $OutDir "settings-smoke.json"
    $env:MUSICOS_RESOLUTION = $Resolution

    $started = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $Exe -PassThru -RedirectStandardOutput $StdoutPath -RedirectStandardError $StderrPath

    $library = Wait-ForResult "library API" {
        Invoke-RestMethod -Uri "$BaseUrl/api/library"
    }
    $readyMs = [Math]::Round($started.Elapsed.TotalMilliseconds, 2)

    $screenshot = Wait-ForResult "first screenshot" {
        $response = Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl/api/screenshot.png"
        if ($response.RawContentLength -gt 1000) { $response.RawContentLength } else { $null }
    }
    $firstScreenshotMs = [Math]::Round($started.Elapsed.TotalMilliseconds, 2)

    $scanWatch = [Diagnostics.Stopwatch]::StartNew()
    $scanDone = Wait-ForResult "library scan" {
        $state = Invoke-RestMethod -Uri "$BaseUrl/api/library"
        if (($state.scan_status -like "Scan complete:*") -or ($state.counts.tracks -ge $TrackCount)) {
            $state
        } else {
            $null
        }
    }
    $scanMs = [Math]::Round($scanWatch.Elapsed.TotalMilliseconds, 2)

    $api = @(
        Measure-Get "/api/status" $Samples
        Measure-Get "/api/library" $Samples
        Measure-Get "/api/diagnostics" $Samples
        Measure-Get "/api/player" $Samples
        Measure-Get "/api/screenshot.png" ([Math]::Max(4, [int]($Samples / 3)))
        Measure-Get "/api/screenshot.bmp" ([Math]::Max(4, [int]($Samples / 3)))
    )
    $input = Measure-Input ([Math]::Max(8, [int]($Samples / 2)))
    $frame = Measure-Frames ([Math]::Max(12, $Samples))
    $final = Invoke-RestMethod -Uri "$BaseUrl/api/library"
    $diagnostics = Invoke-RestMethod -Uri "$BaseUrl/api/diagnostics"
    $process.Refresh()

    $report = [pscustomobject]@{
        generated_at = (Get-Date).ToString("o")
        git_head = (git rev-parse --short HEAD)
        git_dirty = -not [string]::IsNullOrWhiteSpace((git status --porcelain))
        track_count = $TrackCount
        resolution = $Resolution
        ready_ms = $readyMs
        first_screenshot_ms = $firstScreenshotMs
        first_screenshot_bytes = $screenshot
        scan_ms = $scanMs
        scan_status = $final.scan_status
        counts = $final.counts
        indexed_roots = $final.indexed_roots
        cover_art_cache_count = $final.cover_art_cache_count
        frame_ms = $final.frame_ms
        diagnostics_frame_ms = $diagnostics.frame_ms
        frame = $frame
        api = $api
        input = $input
        process = [pscustomobject]@{
            pid = $process.Id
            cpu_seconds = [Math]::Round($process.CPU, 2)
            working_set_mb = [Math]::Round($process.WorkingSet64 / 1MB, 1)
        }
        report_path = $ReportPath
    }

    $report | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 $ReportPath
    Assert-PerfThresholds $report
    $report | ConvertTo-Json -Depth 8
} finally {
    foreach ($name in $oldEnv.Keys) {
        [Environment]::SetEnvironmentVariable($name, $oldEnv[$name], "Process")
    }
    if ($process -and -not $process.HasExited -and -not $KeepRunning) {
        Stop-Process -Id $process.Id -Force
    }
}
