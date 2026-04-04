param(
    [Parameter(Mandatory = $true)]
    [string]$PromotionJson,

    [string]$OutputMarkdown,

    [switch]$FailOnPromotionFail
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $PromotionJson)) {
    throw "Promotion report not found: $PromotionJson"
}

$report = Get-Content -Path $PromotionJson | ConvertFrom-Json

if ($null -eq $report.Window1 -or $null -eq $report.Window2) {
    throw "Invalid promotion JSON: missing Window1/Window2"
}

$promotionPass = [bool]$report.PromotionPass
$window1Pass = [bool]$report.Window1.AllPass
$window2Pass = [bool]$report.Window2.AllPass

$lines = @()
$lines += "# Coding Tools Promotion Summary"
$lines += ""
$lines += ("- Overall Verdict: **{0}**" -f ($(if ($promotionPass) { "PASS" } else { "FAIL" })))
$lines += ("- Window 1: **{0}**" -f ($(if ($window1Pass) { "PASS" } else { "FAIL" })))
$lines += ("- Window 2: **{0}**" -f ($(if ($window2Pass) { "PASS" } else { "FAIL" })))
$lines += ""
$lines += "## Thresholds"
$lines += ""
$lines += ("- Max P95 Regression Percent: {0}" -f $report.Thresholds.MaxP95RegressionPercent)
$lines += ("- Max Stale Reject Increase Percent: {0}" -f $report.Thresholds.MaxStaleRejectIncreasePercent)
$lines += ("- Max Canary Blocked Count: {0}" -f $report.Thresholds.MaxCanaryBlockedCount)
$lines += ""
$lines += "## Window Metrics"
$lines += ""
$lines += "| Metric | Window 1 | Window 2 |"
$lines += "|---|---:|---:|"
$lines += ("| Baseline Median (ms) | {0} | {1} |" -f $report.Window1.BaselineSearchMedianMs, $report.Window2.BaselineSearchMedianMs)
$lines += ("| Canary Median (ms) | {0} | {1} |" -f $report.Window1.CanarySearchMedianMs, $report.Window2.CanarySearchMedianMs)
$lines += ("| Baseline P95 (ms) | {0} | {1} |" -f $report.Window1.BaselineSearchP95Ms, $report.Window2.BaselineSearchP95Ms)
$lines += ("| Canary P95 (ms) | {0} | {1} |" -f $report.Window1.CanarySearchP95Ms, $report.Window2.CanarySearchP95Ms)
$lines += ("| Canary Blocked Count | {0} | {1} |" -f $report.Window1.CanaryBlockedCount, $report.Window2.CanaryBlockedCount)
$lines += ("| Baseline Stale Reject Count | {0} | {1} |" -f $report.Window1.BaselineStaleRejectCount, $report.Window2.BaselineStaleRejectCount)
$lines += ("| Canary Stale Reject Count | {0} | {1} |" -f $report.Window1.CanaryStaleRejectCount, $report.Window2.CanaryStaleRejectCount)
$lines += ""
$lines += "## Recommendation"
$lines += ""
if ($promotionPass) {
    $lines += "Promote coding tools as default-enabled for the next rollout stage. Keep rollback profile ready."
} else {
    $lines += "Hold promotion. Investigate failing gate(s), remediate, and rerun two-window evaluation."
}

$markdown = ($lines -join "`n")

if ($OutputMarkdown) {
    $markdown | Set-Content -Path $OutputMarkdown
    Write-Host "Wrote markdown summary: $OutputMarkdown" -ForegroundColor Green
} else {
    Write-Output $markdown
}

if ($FailOnPromotionFail -and -not $promotionPass) {
    exit 2
}
