# Security Qualification Package

This document defines the automated security qualification that a stable
release requires. It is also a useful brief for an optional independent
review.

Three things close the security gate:

- CI on the exact head commit;
- the technical audit record;
- an explicit disposition (decision) for every critical or high finding.

Qualification includes these tests:

- adversarial process-tree tests: timeout, flooding, and descendants of a
  parent that succeeded;
- topic-stream tests: bounded admission, incremental replay, symlink/identity
  races, slow consumers, and shutdown.

## 1. What to Review

Review these:

- the Rust implementation under `crates/`;
- the generated native hosts under `crates/tachyon-core/src/native/`;
- the public contracts under `api/`.

The archived JavaScript release is **not** in scope. It is not present in this
branch. CI downloads its v26.30.04 executable, after verifying the checksum,
only for the behavioral compatibility differential.

## 2. Start Here

| Document | What it gives you |
| --- | --- |
| [`CONTEXT.md`](../CONTEXT.md) | Domain language, invariants, and non-goals |
| [`THREAT_MODEL.md`](THREAT_MODEL.md) | Assets, trust boundaries, threats, controls, and evidence for each phase |
| [`architecture/OVERVIEW.md`](architecture/OVERVIEW.md) | How the parts fit together |
| [`adr/`](adr/) | Accepted and superseded decisions, with the alternatives they rejected |
| [`PHASE_7_EVIDENCE.md`](PHASE_7_EVIDENCE.md) | Results for fuzzing, sanitizers, drills, soak, and budgets |
| [`SUPPORT_TIERS.md`](SUPPORT_TIERS.md) | What is and is not claimed for each target |

## 3. Trust Boundaries to Examine

| Boundary | Where | Existing assurance |
| --- | --- | --- |
| Application HTML into the compiler | `template/` | Bounded parser; fuzzed |
| Project source discovery and reuse | `project.rs`, `handler/source.rs`, `compiler.rs`, `native/compiler.rs` | One retained capability root; deterministic, bounded traversal that does not follow symlinks; captured bytes for pages, components, shared, config, and server; owned working roots for the compiler and for execution; regression tests for ambient whole-root swaps |
| Untrusted child-process output | `handler/frame.rs` | Length-prefixed protocol; fuzzed for 4.4M (million) executions |
| Handler process lifetime and environment | `handler/process.rs` | Spawned as a process group, never through a shell; environment denied by default; deadlines; cancellation; bounded drain settlement; descendant reaping |
| Environment-selected Firecracker control plane | `handler/isolation.rs` | Complete policy parsing that fails closed; a fixed driver invocation contract for JavaScript/Python; prepared language artifacts rejected before spawn; bounded CPU, memory, and pool values; egress (outgoing traffic) always denied |
| Generated output path handling | `compiler.rs`, `native/host.rs` | Output contained in the project; symlinks rejected; atomic publication |
| Web content inside a native host | `native/{macos,ios,linux,windows,android}.rs` | Ephemeral (temporary) data store; navigation policy; no bridge of any kind |
| Native capability surface | `CapabilityManifest` | Denied by default; `remote_content_bridge` is false |
| Dependency supply chain | `deny.toml`, CI | Advisories, bans, licenses, sources; pinned actions; SBOM (software bill of materials); auditable binary |

## 4. Questions We Want Answered

1. Can any input reach a native `WebSurface` and gain a capability, a bridge,
   or navigation outside its declared origin?
2. Can a handler child process affect the supervisor beyond its declared
   response? Consider frame framing, stream interleaving, resource exhaustion,
   and process lifetime.
3. Can a crafted project escape its own directory during discovery, staging,
   or publication? Consider every supported platform, including Windows path
   semantics.
4. The specifications set bounds: sizes, depths, counts, and deadlines. Are
   these bounds enough to prevent denial of service on a build machine?
5. Can application-authored strings inject code into any generated host
   source? All escaping happens in one place, `native/host.rs`. Is it complete
   for Swift, Java, C, XML, and property-list contexts?
6. Are these acceptable at the tiers claimed for them: the deliberate
   reductions in `PHASE_5_SPEC.md` §6, and the deferred decisions in
   `CONTEXT.md`?
7. Can a partial or conflicting `YON_FIRECRACKER_*` environment configuration
   downgrade silently? Can the external driver exceed the bounded contract
   that the supervisor passes to it?

## 5. Known Gaps, Declared Up Front

We list these here so a reviewer does not spend time finding them again.

- The default process backend runs with the ambient OS capabilities of the
  developer account. The optional Firecracker backend delegates enforcement
  to an external control program that the operator supplies. Tachyon
  validates and supervises the transport. It does not qualify that driver's
  jailer, guest image, snapshots, credentials, or host network policy.
- The Firecracker transport currently supports only JavaScript and Python
  source. TypeScript and prepared direct-language artifacts fail closed before
  the driver starts, because no artifact-transfer contract exists.
- Production HTTP dispatch and middleware exist. They run application
  handlers with the ambient filesystem and network access of the developer
  account.
  - Their public error responses must not expose process diagnostics.
  - Startup binds the initial build, routes, middleware, schedules, and worker
    sources to one immutable Project snapshot. Runtime process mode still
    carries ambient OS capabilities outside that source-selection boundary.
  - The server owns its watcher and scheduled-worker tasks. At shutdown it
    settles them within a bound. A change to a worker schedule requires a
    server restart.
  - The same lifetime token tracks active response producers for streaming
    handlers, hot updates, and topics. Shutdown cancels them before the
    bounded HTTP graceful-close wait. A client disconnect reaps a streaming
    child.
  - The final slice of the global shutdown deadline is reserved for
    abort-and-join settlement of both task registries.
  - Completed response-producer records are reaped continuously while the
    server is live.
- Native signing is ad-hoc or debug-key only, on every platform.
- On Windows, a control has no accessible name separate from its visible text.
  Windows embeds no web view.
- No long-form fuzz campaign with a persistent corpus has run yet.
- The curl/PowerShell bootstrap installers fail closed on release checksums.
  The release workflow verifies Sigstore bundles and GitHub attestations. A
  standalone installer cannot authenticate those proofs unless a verifier is
  already trusted: `cosign` or the GitHub CLI. An authenticated bootstrap
  path remains a medium-severity supply-chain improvement.

## 6. Qualification Record

A completed qualification must state:

- the date;
- the exact commit;
- the scope covered;
- every finding, with its severity, owner, and disposition.

Independent human review may be added here, but it is optional.

### Automated technical pre-review — 2026-08-01

- Reviewer: Codex Security Engineer role #7 (automated qualification).
- Candidate: base commit `2d2fbaf9846d641c86ce4258656562158f08defa`
  plus the current release diff.
- Scope: handler supervision, native WebSurfaces, server error boundaries,
  installers, release/CI workflows, dependencies, and security documentation.
- Initial findings:
  - two high-severity issues: descendant process lifecycle ownership, and
    Linux local-resource navigation;
  - five medium findings: public diagnostic redaction, remote-origin port
    comparison, Unix upgrade atomicity, installer-authentication bootstrap,
    and review-package accuracy.
- Re-review disposition: both high findings and four medium findings are
  closed, with code and regression evidence. Installer-authentication
  bootstrap remains a documented medium improvement. It needs a named owner.
  It is not a critical/high technical release blocker.

This record satisfies the security gate when both conditions hold:

- the final exact-head enterprise qualification passes;
- no critical or high finding is unowned or unresolved.

Optional independent review does not block release.
