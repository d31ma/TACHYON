# Phase 7 Specification: Enterprise Qualification

This document defines the rules for Phase 7. It defines the qualification
evidence that the implementation must produce before any target can move
toward `supported`. It also defines the budgets and drills that make up that
evidence.

Phase 7 adds no feature for applications. It adds proof that the existing
features keep working under hostile input, sustained load, interruption, and
audit.

## 1. Fuzzing

Every input that crosses a trust boundary has a fuzz target under `fuzz/`.

| Target | Boundary |
| --- | --- |
| `template_frontend` | Control tags, bindings, and component references. |
| `handler_frame` | Length-prefixed frames read from an untrusted child process. |
| `native_planner` | Resolved HTML converted into Native UI v1 for five targets. |

Requirements:

- Every target runs under `AddressSanitizer`.
- No input may cause a panic, an abort, a timeout, or memory growth without a
  limit.
- `native_planner` also checks a rule across platforms: all five targets must
  agree on whether an input is valid. A disagreement is a bug even when
  neither side crashes.
- The repository tracks hand-written seeds. It does not track generated
  corpus entries.
- CI (continuous integration) publishes a crashing input as an artifact. The
  input then becomes a regression test.

The fuzz crate needs a nightly toolchain, so it is excluded from the pinned
workspace. It is never shipped.

## 2. Sanitizers

The library suite runs under `AddressSanitizer`, `LeakSanitizer`, and
`ThreadSanitizer`. Our own crates set `unsafe_code = "forbid"`. So the
sanitizers mainly test dependency code and the process, threading, and
filesystem operations that the implementation performs.

## 3. The `fuzzing` Feature

`tachyon-core` gives fuzz targets access to its crate-private parsers and
decoders through the `fuzzing` feature. This is not a stable API. It has no
compatibility guarantee. A released artifact must never enable it.

## 4. Recovery Drills

Each drill states a property that an operator can see and the failure that
the drill simulates.

| Drill | Property |
| --- | --- |
| Interrupted build | When a build is killed while it runs, the published output stays byte-identical and gets no new files. The next build succeeds. |
| Corrupted incremental cache | The build detects damaged build state and does not trust it. The rebuild succeeds and produces the same output. |
| Failed build | A regression in the source never damages the published application. |
| Concurrent builds | Builds that run at the same time never publish a half-written application. Every route in the manifest has its document. |
| Read-only output | A target that cannot be written produces a diagnostic. It never causes a panic or a partial write. |

Publication is atomic. Every drill checks that property from outside, against
the real executable.

## 5. Soak

Many rebuilds in one long-lived working tree must keep three properties:

1. the output stays deterministic in every iteration;
2. the number of open file descriptors does not grow beyond a small constant;
3. latency does not rise as state builds up.

`TAC_SOAK_ITERATIONS` sets the iteration count. The default is 24 locally. CI
uses a higher value.

## 6. Performance Budgets

The limits are wide on purpose, because shared CI hardware gives noisy
results. They catch a regression that is about ten times worse, not one of a
few percent.

| Budget | Ceiling |
| --- | --- |
| Clean build, 50 routes | 20 s |
| Incremental rebuild, 50 routes | 20 s |
| Generated output per route | 64 KiB |

A budget may become stricter when measurements support it. Making a budget
looser requires a recorded reason.

## 7. Supply-Chain Evidence

- `cargo deny check advisories bans licenses sources` must pass for every
  change.
- Every GitHub Action is pinned to a commit that cannot change.
- CI generates a CycloneDX software bill of materials for the workspace.
- The release binary is built with `cargo auditable`. This embeds its
  dependency list, so a deployed artifact can be audited without its source
  tree.

## 8. Automated Security Qualification

Phase 7 requires automated qualification of the trust boundaries, the threat
model, the contracts, the dependencies, and the release tools. Every critical
and high finding must have an owner and a decision before a stable tag is
created.

We encourage independent human review for more assurance. It is not a gate
for merges, support tiers, tags, or publication.

## 9. Exit Gate

- [x] Every trust boundary has a fuzz target that runs without failures under
      `AddressSanitizer`.
- [x] The `native_planner` rule across targets holds under fuzzing.
- [x] The library suite passes under `AddressSanitizer`.
- [x] Every recovery drill passes against the real executable.
- [x] The soak drill keeps the determinism, descriptor, and latency properties.
- [x] Every performance budget is met and published.
- [x] CI has jobs for the supply-chain policy, the SBOM (software bill of
      materials), and the auditable build.
- [x] Automated security qualification is complete. No critical or high
      finding remains without an owner.
- [x] Install, upgrade, rollback, and uninstall exercises are recorded for the
      standalone `ty` artifact. Native targets still have their own evidence
      requirements for promotion.

Lifecycle evidence for native targets is still required to promote those
targets. It does not block publication of the standalone CLI.
