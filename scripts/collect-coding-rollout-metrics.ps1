param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineLog,

    [Parameter(Mandatory = $true)]
    [string]$CanaryLog,

    [double]$MaxP95RegressionPercent = 10,

    [double]$MaxStaleRejectIncreasePercent = 20,

    [int]$MaxCanaryBlockedCount = 0,

    [switch]$FailOnGateViolation,

    [switch]$ReturnGateStatus,

    [string]$OutputJson
)

$ErrorActionPreference = "Stop"

function Get-Percentile {
    param(
        [double[]]$Values,
        [double]$P
    )

    if (-not $Values -or $Values.Count -eq 0) {
        return [double]::NaN
    }

    $sorted = $Values | Sort-Object
    $idx = [int][math]::Ceiling($P * $sorted.Count) - 1
    if ($idx -lt 0) { $idx = 0 }
    if ($idx -ge $sorted.Count) { $idx = $sorted.Count - 1 }
    return [double]$sorted[$idx]
}

function Parse-CodingMetrics {
    param(
        [string]$Path,
        [string]$Label
    )

    if (-not (Test-Path $Path)) {
        throw "Log file not found: $Path"
    }

    $lines = Get-Content -Path $Path

    $grepElapsed = @()
    $fileElapsed = @()
    $blockedCount = 0
    $staleRejectCount = 0

    foreach ($line in $lines) {
        $m = [regex]::Match($line, 'tool="?(?<tool>[a-z_]+)"?.*elapsed_ms=(?<ms>[0-9]+)')
        if ($m.Success) {
            $tool = $m.Groups['tool'].Value
            $ms = [double]$m.Groups['ms'].Value
            if ($tool -eq 'grep_search') {
                $grepElapsed += $ms
            } elseif ($tool -eq 'file_search') {
                $fileElapsed += $ms
            }
        }

        if ($line -match 'Coding tool blocked by exec_policy\.enable_coding_tools=false') {
            $blockedCount++
        }
        if ($line -match 'Read-before-write precondition failed') {
            $staleRejectCount++
        }
    }

    $allSearch = @($grepElapsed + $fileElapsed)

    [pscustomobject]@{
        Profile = $Label
        SearchEvents = $allSearch.Count
        SearchMedianMs = if ($allSearch.Count -gt 0) { [math]::Round((Get-Percentile -Values $allSearch -P 0.50), 3) } else { $null }
        SearchP95Ms = if ($allSearch.Count -gt 0) { [math]::Round((Get-Percentile -Values $allSearch -P 0.95), 3) } else { $null }
        GrepEvents = $grepElapsed.Count
        GrepMedianMs = if ($grepElapsed.Count -gt 0) { [math]::Round((Get-Percentile -Values $grepElapsed -P 0.50), 3) } else { $null }
        FileEvents = $fileElapsed.Count
        FileMedianMs = if ($fileElapsed.Count -gt 0) { [math]::Round((Get-Percentile -Values $fileElapsed -P 0.50), 3) } else { $null }
        CodingToolBlockedCount = $blockedCount
        StaleWriteRejectCount = $staleRejectCount
    }
}

function Get-PercentIncrease {
    param(
        [double]$Baseline,
        [double]$Canary
    )

    if ($Baseline -eq 0) {
        if ($Canary -eq 0) {
            return 0.0
        }
        return [double]::PositiveInfinity
    }

    return (($Canary - $Baseline) / $Baseline) * 100.0
}

$baseline = Parse-CodingMetrics -Path $BaselineLog -Label "baseline"
$canary = Parse-CodingMetrics -Path $CanaryLog -Label "canary"

$summary = @($baseline, $canary)
$summary | Format-Table -AutoSize

if ($baseline.SearchMedianMs -and $canary.SearchMedianMs) {
    $speedup = [math]::Round($baseline.SearchMedianMs / $canary.SearchMedianMs, 3)
    Write-Host ""
    Write-Host ("Canary search median speedup (baseline/canary): {0}x" -f $speedup) -ForegroundColor Green
}

Write-Host ""
Write-Host "Promotion Gate Evaluation" -ForegroundColor Cyan

$gateResults = @()

# Gate 1: Median must not regress.
$medianGatePass = $false
$medianGateDetail = "Insufficient search events to evaluate median gate"
if ($null -ne $baseline.SearchMedianMs -and $null -ne $canary.SearchMedianMs) {
    $medianGatePass = ($canary.SearchMedianMs -le $baseline.SearchMedianMs)
    $medianGateDetail = "baseline={0}ms canary={1}ms" -f $baseline.SearchMedianMs, $canary.SearchMedianMs
}
$gateResults += [pscustomobject]@{
    Gate = "search_median_non_regression"
    Pass = $medianGatePass
    Detail = $medianGateDetail
}

# Gate 2: P95 regression <= configured threshold.
$p95GatePass = $false
$p95GateDetail = "Insufficient search events to evaluate p95 gate"
if ($null -ne $baseline.SearchP95Ms -and $null -ne $canary.SearchP95Ms) {
    $p95IncreasePct = Get-PercentIncrease -Baseline $baseline.SearchP95Ms -Canary $canary.SearchP95Ms
    $p95GatePass = ($p95IncreasePct -le $MaxP95RegressionPercent)
    $p95GateDetail = "baseline={0}ms canary={1}ms increase={2}% limit={3}%" -f $baseline.SearchP95Ms, $canary.SearchP95Ms, ([math]::Round($p95IncreasePct, 3)), $MaxP95RegressionPercent
}
$gateResults += [pscustomobject]@{
    Gate = "search_p95_regression_limit"
    Pass = $p95GatePass
    Detail = $p95GateDetail
}

# Gate 3: Canary blocked count near zero.
$blockedGatePass = ($canary.CodingToolBlockedCount -le $MaxCanaryBlockedCount)
$blockedGateDetail = "canary={0} limit={1}" -f $canary.CodingToolBlockedCount, $MaxCanaryBlockedCount
$gateResults += [pscustomobject]@{
    Gate = "canary_blocked_count_limit"
    Pass = $blockedGatePass
    Detail = $blockedGateDetail
}

# Gate 4: Stale rejection increase within threshold.
$staleIncreasePct = Get-PercentIncrease -Baseline $baseline.StaleWriteRejectCount -Canary $canary.StaleWriteRejectCount
$staleGatePass = ($staleIncreasePct -le $MaxStaleRejectIncreasePercent)
$staleGateDetail = "baseline={0} canary={1} increase={2}% limit={3}%" -f $baseline.StaleWriteRejectCount, $canary.StaleWriteRejectCount, ([math]::Round($staleIncreasePct, 3)), $MaxStaleRejectIncreasePercent
$gateResults += [pscustomobject]@{
    Gate = "stale_reject_increase_limit"
    Pass = $staleGatePass
    Detail = $staleGateDetail
}

$gateResults | Format-Table -AutoSize

$allPass = ($gateResults | Where-Object { -not $_.Pass }).Count -eq 0
if ($allPass) {
    Write-Host "Gate verdict: PASS (eligible for promotion window)" -ForegroundColor Green
} else {
    Write-Host "Gate verdict: FAIL (hold promotion and investigate)" -ForegroundColor Red
    if ($FailOnGateViolation) {
        exit 2
    }
}

if ($OutputJson) {
    $report = [pscustomobject]@{
        Baseline = $baseline
        Canary = $canary
        Gates = $gateResults
        AllPass = $allPass
        Thresholds = [pscustomobject]@{
            MaxP95RegressionPercent = $MaxP95RegressionPercent
            MaxStaleRejectIncreasePercent = $MaxStaleRejectIncreasePercent
            MaxCanaryBlockedCount = $MaxCanaryBlockedCount
        }
    }
    $report | ConvertTo-Json -Depth 6 | Set-Content -Path $OutputJson
}

if ($ReturnGateStatus) {
    [pscustomobject]@{
        AllPass = $allPass
        BaselineSearchMedianMs = $baseline.SearchMedianMs
        CanarySearchMedianMs = $canary.SearchMedianMs
        BaselineSearchP95Ms = $baseline.SearchP95Ms
        CanarySearchP95Ms = $canary.SearchP95Ms
        CanaryBlockedCount = $canary.CodingToolBlockedCount
        BaselineStaleRejectCount = $baseline.StaleWriteRejectCount
        CanaryStaleRejectCount = $canary.StaleWriteRejectCount
    }
}
