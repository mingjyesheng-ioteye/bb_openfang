param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineLogWindow1,

    [Parameter(Mandatory = $true)]
    [string]$CanaryLogWindow1,

    [Parameter(Mandatory = $true)]
    [string]$BaselineLogWindow2,

    [Parameter(Mandatory = $true)]
    [string]$CanaryLogWindow2,

    [double]$MaxP95RegressionPercent = 10,

    [double]$MaxStaleRejectIncreasePercent = 20,

    [int]$MaxCanaryBlockedCount = 0,

    [string]$OutputJson
)

$ErrorActionPreference = "Stop"

$scriptPath = Join-Path $PSScriptRoot "collect-coding-rollout-metrics.ps1"

function Invoke-WindowEvaluation {
    param(
        [string]$WindowLabel,
        [string]$BaselineLog,
        [string]$CanaryLog
    )

    if (-not (Test-Path $BaselineLog)) {
        throw "[$WindowLabel] Baseline log not found: $BaselineLog"
    }
    if (-not (Test-Path $CanaryLog)) {
        throw "[$WindowLabel] Canary log not found: $CanaryLog"
    }

    Write-Host "" 
    Write-Host "[$WindowLabel] Evaluating promotion gates..." -ForegroundColor Cyan

    $status = & $scriptPath `
        -BaselineLog $BaselineLog `
        -CanaryLog $CanaryLog `
        -MaxP95RegressionPercent $MaxP95RegressionPercent `
        -MaxStaleRejectIncreasePercent $MaxStaleRejectIncreasePercent `
        -MaxCanaryBlockedCount $MaxCanaryBlockedCount `
        -ReturnGateStatus

    if ($null -eq $status) {
        throw "[$WindowLabel] Collector did not return gate status"
    }

    return $status
}

$window1Status = Invoke-WindowEvaluation -WindowLabel "window-1" -BaselineLog $BaselineLogWindow1 -CanaryLog $CanaryLogWindow1
$window2Status = Invoke-WindowEvaluation -WindowLabel "window-2" -BaselineLog $BaselineLogWindow2 -CanaryLog $CanaryLogWindow2

$window1Pass = [bool]$window1Status.AllPass
$window2Pass = [bool]$window2Status.AllPass

$promotionPass = ($window1Pass -and $window2Pass)

if ($OutputJson) {
    $combined = [pscustomobject]@{
        Window1 = $window1Status
        Window2 = $window2Status
        PromotionPass = $promotionPass
        Thresholds = [pscustomobject]@{
            MaxP95RegressionPercent = $MaxP95RegressionPercent
            MaxStaleRejectIncreasePercent = $MaxStaleRejectIncreasePercent
            MaxCanaryBlockedCount = $MaxCanaryBlockedCount
        }
    }
    $combined | ConvertTo-Json -Depth 6 | Set-Content -Path $OutputJson
}

Write-Host ""
if ($promotionPass) {
    Write-Host "Promotion verdict: PASS (both windows passed all gates)" -ForegroundColor Green
    exit 0
}

Write-Host "Promotion verdict: FAIL (one or more windows failed gates)" -ForegroundColor Red
exit 2
