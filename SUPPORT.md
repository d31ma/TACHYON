# Tachyon Support

## Current Release Status

The Rust rewrite has finished engineering phases 0–7. It has also finished
later work on the client, on server isolation and on the native runtime.
Version `26.35.07` is a release candidate. It is not a supported release or a
preview release of the application framework. Its artifacts come with no
promise about compatibility, uptime, security support or production use.
These documents record the current evidence and the gates that are still left
before promotion: `docs/PROJECT_PLAN.md`, `docs/CUTOVER.md` and
`docs/SUPPORT_TIERS.md`.

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
