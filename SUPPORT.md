# Tachyon Support

## Current Release Status

The Rust rewrite has finished engineering phases 0–7. It has also finished
later work on the client, on server isolation and on the native runtime.
26.45.07 is a release candidate. A release
candidate is not a supported release or a preview release of the application
framework. No target is promoted to `supported`, in this candidate or in the
published versions up to `26.36.01`. Its artifacts come with no promise about
compatibility, uptime, security support or production use.

[`docs/CUTOVER.md`](docs/CUTOVER.md) lists the release-candidate gates, the
ones this candidate has passed and the ones it has not.
[`docs/SUPPORT_TIERS.md`](docs/SUPPORT_TIERS.md) records each target's tier
and the evidence a target needs before promotion.
[`docs/PROJECT_PLAN.md`](docs/PROJECT_PLAN.md) has the plan.

Use:

- GitHub issues for public bugs that you can reproduce, and for feature
  proposals;
- GitHub discussions for questions about usage and architecture;
- private vulnerability reporting for security issues.

When you open an issue, include:

- the Tachyon version or commit;
- the operating system and architecture;
- the versions of the toolchains that matter;
- the smallest reproduction;
- the actual result and the expected result;
- diagnostics with private data removed.

Platform status uses only the words defined in `docs/SUPPORT_TIERS.md`. Do not
describe a platform as supported when it is listed as buildable or
simulator-tested.
