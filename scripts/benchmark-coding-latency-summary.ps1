param(
    [int]$Runs = 5
)

$ErrorActionPreference = "Stop"

if ($Runs -lt 1) {
    throw "Runs must be >= 1"
}

function Get-StatSummary {
    param(
        [double[]]$Seq,
        [double[]]$Par
    )

    if ($Seq.Count -ne $Par.Count -or $Seq.Count -eq 0) {
        throw "Invalid metric arrays"
    }

    $n = $Seq.Count
    $seqSorted = $Seq | Sort-Object
    $parSorted = $Par | Sort-Object
    $speedups = for ($i = 0; $i -lt $n; $i++) {
        if ($Par[$i] -le 0) { 0.0 } else { $Seq[$i] / $Par[$i] }
    }
    $speedupSorted = $speedups | Sort-Object

    $medianIndexA = [int][math]::Floor(($n - 1) / 2)
    $medianIndexB = [int][math]::Ceiling(($n - 1) / 2)
    $p95Index = [int][math]::Ceiling(0.95 * $n) - 1
    if ($p95Index -lt 0) { $p95Index = 0 }
    if ($p95Index -ge $n) { $p95Index = $n - 1 }

    [pscustomobject]@{
        Runs = $n
        SeqMedianMs = [math]::Round((($seqSorted[$medianIndexA] + $seqSorted[$medianIndexB]) / 2.0), 3)
        ParMedianMs = [math]::Round((($parSorted[$medianIndexA] + $parSorted[$medianIndexB]) / 2.0), 3)
        SeqP95Ms = [math]::Round($seqSorted[$p95Index], 3)
        ParP95Ms = [math]::Round($parSorted[$p95Index], 3)
        SpeedupMedianX = [math]::Round((($speedupSorted[$medianIndexA] + $speedupSorted[$medianIndexB]) / 2.0), 3)
        SpeedupP95X = [math]::Round($speedupSorted[$p95Index], 3)
    }
}

function Invoke-BenchmarkAndParse {
    param(
        [string]$TestName,
        [string]$Pattern
    )

    $cmd = "cargo test -p openfang-runtime --lib $TestName -- --ignored --nocapture 2>&1"
    $output = & cmd /c $cmd
    if ($LASTEXITCODE -ne 0) {
        throw "Benchmark test failed: $TestName"
    }

    $line = ($output | Select-String -Pattern $Pattern | Select-Object -First 1)
    if (-not $line) {
        throw "Could not parse benchmark output for $TestName"
    }

    $m = [regex]::Match($line.ToString(), 'seq=(?<seq>[0-9.]+)ms, par=(?<par>[0-9.]+)ms')
    if (-not $m.Success) {
        throw "Timing pattern not found for $TestName"
    }

    [pscustomobject]@{
        SeqMs = [double]$m.Groups['seq'].Value
        ParMs = [double]$m.Groups['par'].Value
    }
}

$syntheticSeq = @()
$syntheticPar = @()
$scenarioSeq = @()
$scenarioPar = @()
$compactionElapsed = @()

Write-Host "Running coding latency summary over $Runs run(s)..." -ForegroundColor Cyan
for ($i = 1; $i -le $Runs; $i++) {
    Write-Host ("Run {0}/{1}" -f $i, $Runs) -ForegroundColor Yellow

    $synthetic = Invoke-BenchmarkAndParse `
        -TestName "benchmark_parallel_read_only_batch_latency" `
        -Pattern "benchmark_parallel_read_only_batch_latency"

    $scenario = Invoke-BenchmarkAndParse `
        -TestName "benchmark_search_tools_parallel_vs_sequential" `
        -Pattern "benchmark_search_tools_parallel_vs_sequential"

    $compactionCmd = "cargo test -p openfang-runtime --lib benchmark_compaction_long_session_latency -- --ignored --nocapture 2>&1"
    $compactionOutput = & cmd /c $compactionCmd
    if ($LASTEXITCODE -ne 0) {
        throw "Benchmark test failed: benchmark_compaction_long_session_latency"
    }
    $compactionLine = ($compactionOutput | Select-String -Pattern "benchmark_compaction_long_session_latency" | Select-Object -First 1)
    if (-not $compactionLine) {
        throw "Could not parse benchmark output for benchmark_compaction_long_session_latency"
    }
    $cm = [regex]::Match($compactionLine.ToString(), 'elapsed=(?<elapsed>[0-9]+)ms')
    if (-not $cm.Success) {
        throw "Compaction timing pattern not found"
    }

    $syntheticSeq += $synthetic.SeqMs
    $syntheticPar += $synthetic.ParMs
    $scenarioSeq += $scenario.SeqMs
    $scenarioPar += $scenario.ParMs
    $compactionElapsed += [double]$cm.Groups['elapsed'].Value
}

$syntheticSummary = Get-StatSummary -Seq $syntheticSeq -Par $syntheticPar
$scenarioSummary = Get-StatSummary -Seq $scenarioSeq -Par $scenarioPar
$compactionSorted = $compactionElapsed | Sort-Object
$nCompaction = $compactionSorted.Count
$cMedianA = [int][math]::Floor(($nCompaction - 1) / 2)
$cMedianB = [int][math]::Ceiling(($nCompaction - 1) / 2)
$cP95 = [int][math]::Ceiling(0.95 * $nCompaction) - 1
if ($cP95 -lt 0) { $cP95 = 0 }
if ($cP95 -ge $nCompaction) { $cP95 = $nCompaction - 1 }

$compactionSummary = [pscustomobject]@{
    Runs = $nCompaction
    CompactionMedianMs = [math]::Round((($compactionSorted[$cMedianA] + $compactionSorted[$cMedianB]) / 2.0), 3)
    CompactionP95Ms = [math]::Round($compactionSorted[$cP95], 3)
}

Write-Host ""
Write-Host "Synthetic Batch Benchmark Summary" -ForegroundColor Green
$syntheticSummary | Format-Table -AutoSize

Write-Host ""
Write-Host "Scenario Search Benchmark Summary" -ForegroundColor Green
$scenarioSummary | Format-Table -AutoSize

Write-Host ""
Write-Host "Long-session Compaction Benchmark Summary" -ForegroundColor Green
$compactionSummary | Format-Table -AutoSize
