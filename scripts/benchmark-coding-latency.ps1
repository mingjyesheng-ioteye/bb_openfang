param(
    [switch]$NoCapture
)

$ErrorActionPreference = "Stop"

Write-Host "Running coding latency benchmark harnesses..." -ForegroundColor Cyan

$baseArgs = @("test", "-p", "openfang-runtime", "--lib")
$captureArgs = if ($NoCapture) { @("--", "--ignored") } else { @("--", "--ignored", "--nocapture") }

Write-Host "[1/3] Synthetic parallel batch benchmark" -ForegroundColor Yellow
& cargo @baseArgs "benchmark_parallel_read_only_batch_latency" @captureArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[2/3] Scenario search benchmark" -ForegroundColor Yellow
& cargo @baseArgs "benchmark_search_tools_parallel_vs_sequential" @captureArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[3/3] Long-session compaction benchmark" -ForegroundColor Yellow
& cargo @baseArgs "benchmark_compaction_long_session_latency" @captureArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Benchmark harness run complete." -ForegroundColor Green
