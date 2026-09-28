# Phase 7 Evidence

This is the validation record for [`PHASE_7_SPEC.md`](PHASE_7_SPEC.md).

Machine: macOS 26 (Darwin 25.5.0), Apple silicon, Rust 1.97.1,
nightly-2026-07-20, cargo-fuzz 0.13.1.

## 1. Fuzzing

```bash
cargo +nightly-2026-07-20 fuzz run <target> -- -max_total_time=45 -rss_limit_mb=2048 -timeout=10
```

Every target runs under `AddressSanitizer`. `cargo fuzz` enables it by
default.

| Target | Executions | Coverage | Corpus | Crashes | Timeouts | OOMs |
| --- | --- | --- | --- | --- | --- | --- |
| `html_frontend` | 1,629,949 | 2,946 | 2,737 | 0 | 0 | 0 |
| `template_frontend` | 887,704 | 3,816 | 3,121 | 0 | 0 | 0 |
| `handler_frame` | 4,482,194 | 1,152 | 1,031 | 0 | 0 | 0 |
| `native_planner` | 197,845 | 3,559 | 1,598 | 0 | 0 | 0 |
| **total** | **7,197,692** | | **8,487** | **0** | **0** | **0** |

`handler_frame` is the highest-value target. It decodes length-prefixed
frames that an untrusted child process writes. It sustained 97,000 executions
per second. It never produced anything other than a decoded response or a
protocol diagnostic.

`native_planner` also asserts that all five native targets agree on whether an
input is valid. That invariant held across 197,845 inputs. This is mechanical
evidence for the claim in Phase 5 spec §1.2: platform selection never changes
what the planner accepts.

CI runs each target for 300 seconds per pull request. It publishes any
crashing input as an artifact.

The first such CI campaign found a real defect that the shorter local runs
above had missed:

- `template_frontend` reached a 78-byte input. In that input, malformed token
  recovery left `<slot>` open on the parser stack.
- Appending the next child then hit an `unreachable!()` in
  `OpenNode::children_mut`, and the parser panicked.

The fix: `<slot>` is a leaf, so the parser now refuses the child with
diagnostic 1301. It no longer reparents or aborts. The impossible-state panic
is gone, not merely unreachable.

- The exact crashing bytes are a unit regression test in
  `crates/tachyon-core/src/template/frontend.rs`.
- `fuzz/corpus/template_frontend/seed-open-slot` keeps the shape in the
  permanent corpus.
- Minimization cannot reduce the artifact any further, because it no longer
  crashes.

After the fix, the artifact replays cleanly. A 120-second `template_frontend`
campaign completed 1,932,958 executions with no crash. Alongside it, 60-second
campaigns ran on the other three targets: 1,948,319, 5,257,667, and 307,471
executions, all clean.

## 2. Sanitizers

```bash
RUSTFLAGS="-Zsanitizer=address" cargo +nightly-2026-07-20 test -p tachyon-core --lib \
  --target aarch64-apple-darwin
```

Result: `85 passed; 0 failed`. No `AddressSanitizer` report.

`LeakSanitizer` and `ThreadSanitizer` are not available for
`aarch64-apple-darwin`. The `sanitizers` CI job runs all three on
`x86_64-unknown-linux-gnu`. That job passed on pull request run
[30715759023](https://github.com/d31ma/Tachyon/actions/runs/30715759023).

## 3. Recovery Drills

```bash
cargo test --locked -p tachyon-cli --test it -- phase7_cli::
```

Result: `7 passed; 0 failed`.

| Drill | Simulated failure | Observed |
| --- | --- | --- |
| Interrupted build | 5 builds of a 40-route project, each killed 15 ms after it started | Published manifest and file count unchanged; the next build succeeded and reproduced the same output |
| Corrupted incremental cache | Build state overwritten with empty, truncated, partial-JSON, NUL, and wrong-type content | The build recovered from every corruption; published output was byte-identical each time |
| Failed build | A valid application rebuilt from an orphaned `<logic :else>` | The build failed and the published `index.html` was untouched |
| Concurrent builds | 4 non-incremental builds racing on one output directory | The route manifest listed all 12 routes and every route's document existed |
| Read-only output | Output directory set to mode `0500` | The failure carried a `TY` diagnostic code; no panic |

## 4. Soak

24 sequential rebuilds of a 10-route project in one working tree (200 in CI).

| Property | Result |
| --- | --- |
| Deterministic output | Every iteration reproduced the baseline route manifest |
| Descriptor growth | Within the allowed constant |
| Latency drift | Second-half mean stayed within the bound relative to first-half mean |

## 5. Performance Budgets

50-route project, debug binary, on the machine above.

| Budget | Ceiling | Result |
| --- | --- | --- |
| Clean build | 20 s | Met |
| Incremental rebuild | 20 s | Met |
| Output per route | 64 KiB | Met |

The ceilings are wide by design. They catch order-of-magnitude regressions,
not percentage drift. We measure the debug binary deliberately, because that
is what CI builds.

## 6. Supply-Chain Evidence

| Control | State |
| --- | --- |
| `cargo deny check advisories bans licenses sources` | Gates every change in the `supply-chain` job |
| GitHub Action pinning | Every action pinned to an immutable commit; enforced by `repository_policy` |
| CycloneDX SBOM (software bill of materials) | Generated by the `provenance` job and published as an artifact |
| Auditable release binary | `cargo auditable build --release --bin ty` embeds the dependency list |

## 7. Release Lifecycle

```bash
./scripts/release/lifecycle-drill.sh
```

Result: `PASS: release lifecycle drill`.

- Two builds of the same source produced a bit-identical binary.
- The published checksums verified.
- A one-byte tamper failed verification.
- The isolated install reported the exact `VERSION`.
- That installed binary built a real project.
- Upgrade, rollback, and uninstall then passed.
- Uninstall left no file beneath the isolated prefix.

This is local release engineering evidence. It does not replace one
tag-driven workflow run that publishes attested archives.

## 8. Open Gaps

| Gap | Why it is open | What closes it |
| --- | --- | --- |
| **Tag-driven release provenance** | The local lifecycle and reproducible artifact drill pass. The tag workflow now stages before publication and runs native verification after upload. But no real tag run has produced evidence yet. | Run the release workflow from a qualified immutable tag. Its five native verification jobs must pass before publication. |
| **Windows UI Automation roles** | Native execution now passes in CI. UIA (UI Automation) names reach the generated controls. A real `Button` HWND (window handle) backs the button. Native `BM_CLICK` changes state. The lifecycle completes. However, the hosted managed UIA client still reports every standard child control as `ControlType.Pane`. So there is no evidence yet for semantic roles or accessibility-pattern activation, and Windows is not promoted to `native-tested`. | A client or generated provider that exposes the standard HWNDs with their semantic UIA roles and `InvokePattern`. Then a role assertion must pass in the Windows gate. |
| First recorded long-form persistent-corpus campaign | `fuzz-scheduled.yml` now runs every target for 30 minutes each by default. It restores and saves an evolving corpus. It publishes the corpus and any crash for audit. No scheduled run has reported yet. | The first successful scheduled run, and then continued weekly execution. |

The `fuzz`, `sanitizers`, `soak`, and `provenance` jobs, including Linux
LeakSanitizer and ThreadSanitizer, passed on pull request run
[30715759023](https://github.com/d31ma/Tachyon/actions/runs/30715759023).

This document promotes no target to `supported`. Automated security
qualification is complete. Tag-driven release provenance remains open until
the private staged workflow produces evidence. That provenance must pass
before public promotion.
