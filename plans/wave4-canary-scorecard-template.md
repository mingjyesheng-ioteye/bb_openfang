# Wave 4 Canary Scorecard Template

Date: 2026-04-04
Release Candidate: staged-working-tree (pre-commit)
Evaluators: mingj, copilot

## 1) Input Windows

- Window 1 baseline log: PENDING
- Window 1 canary log: PENDING
- Window 2 baseline log: PENDING
- Window 2 canary log: PENDING

Discovery note (2026-04-04): auto-scan in repo/home found no baseline/canary/rollout log candidates outside build artifacts.

## 2) Gate Thresholds

- Max P95 regression (%): 10
- Max stale reject increase (%): 20
- Max canary blocked count: 0

## 3) Automated Gate Verdict

Run:

```powershell
./scripts/evaluate-coding-promotion.ps1 \
  -BaselineLogWindow1 <path> \
  -CanaryLogWindow1 <path> \
  -BaselineLogWindow2 <path> \
  -CanaryLogWindow2 <path> \
  -OutputJson <path>
```

- PromotionPass: PENDING (requires baseline/canary logs)
- Window 1 status: PENDING
- Window 2 status: PENDING

## 4) SLO Snapshot

- Search median (baseline -> canary): PENDING
- Search P95 (baseline -> canary): PENDING
- Coding tool blocked count (canary): PENDING
- Stale write reject count (baseline -> canary): PENDING
- Compaction median latency: 0 ms (local benchmark, runs=3)
- Compaction P95 latency: 0 ms (local benchmark, runs=3)

Local benchmark snapshot (scripts/benchmark-coding-latency-summary.ps1 -Runs 3):

- Synthetic batch: seq median 365.404 ms, par median 47.862 ms, speedup median 7.635x
- Synthetic batch: seq P95 365.642 ms, par P95 48.043 ms, speedup P95 7.989x
- Scenario search: seq median 120.848 ms, par median 40.817 ms, speedup median 2.961x
- Scenario search: seq P95 155.205 ms, par P95 42.17 ms, speedup P95 3.68x

## 5) Failure Taxonomy Summary

Top tool failure codes in canary window:

- PENDING (requires canary log parsing from tool_execution_slo events)

Retry classes observed:

- retryable: PENDING
- no_retry: PENDING
- retry_after_reconnect: PENDING
- retry_after_config: PENDING

## 6) MCP Operability Checks

- MCP connected server count at startup: PENDING
- MCP diagnostics checks executed: PENDING
- MCP auth/config incidents: PENDING
- MCP fallback hint effectiveness notes: PENDING

## 7) Decision

- Final decision: HOLD (await canary window evidence)
- Reason: Local benchmarks are strong, but promotion gates require baseline/canary log windows.
- Required follow-ups:
  - Collect baseline/canary logs for two windows and run evaluate-coding-promotion.ps1.
  - Fill failure taxonomy and MCP operability counts from production-like logs.
