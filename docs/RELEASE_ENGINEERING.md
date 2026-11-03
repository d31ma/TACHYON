# Tachyon Rust Release Engineering

## Release Boundary

Engineering phases 0–7 implement:

- the compiler;
- the supervised server boundary;
- view semantics and islands;
- five native host families;
- migration tooling;
- automation for enterprise qualification.

`VERSION` records the product release identity. The internal Cargo package
identity stays `0.0.0-phase4`. This way, nobody can mistake package metadata
for the public product version.

Stable Tachyon releases keep the existing product identity, which uses UTC
CalVer (calendar versioning). Internal Rust package versions are
implementation metadata. They do not replace the product version in `VERSION`
and in the artifact manifest. The build embeds `VERSION` at compile time in
`ty --version` and in native artifact manifests. So when the release identity
changes, the executable cannot keep reporting the internal Cargo version.

## Branch Model

- The Rust cutover is complete. `develop` is the integration branch, and
  `main` is the release branch.
- Feature and fix branches use `feat/*` or `fix/*` and start from
  `origin/develop`. After their local gates pass, they are fast-forwarded into
  `develop`.
- A release reaches `main` through a pull request from `develop`. That pull
  request is the only time GitHub Actions runs the full CI.
- Release branches use `release/<UTC-CalVer>`. Create them from the updated
  remote default branch after the implementation is merged. Keep them forever.
- Keep old rewrite branches and local-recovery branches for comparison. Do
  not replace all of main with a recovered source tree.
- Never force-push release history. Never move an existing version tag.

## Release Inputs

Every release comes from:

- an immutable commit and an annotated tag;
- the exact `VERSION`;
- the committed `Cargo.lock`;
- the exact `rust-toolchain.toml`;
- pinned GitHub Actions;
- canonical contract versions;
- clean, temporary native builders;
- an explicit `SOURCE_DATE_EPOCH`.

## Required Artifacts

Each target archive contains:

- `ty` or `ty.exe`;
- license and third-party notices;
- installation notes;
- `manifest.json`;
- contract compatibility metadata.

The release contains:

- SHA-256 checksums that fail closed (any mismatch counts as a failure);
- a CycloneDX SBOM (software bill of materials);
- signed build provenance;
- keyless Sigstore signatures bound to the release workflow;
- release notes for people to read.

## Release CI

The Rust release workflow runs in CI (continuous integration). A tag starts
it, and it never creates its own tag. It:

1. verifies the tag, `VERSION`, the changelog, and the commit state;
2. runs all CI and compatibility gates;
3. builds on native runners for each target;
4. runs the produced binary on each runner;
5. packages deterministic target archives;
6. runs the [lifecycle drill](#lifecycle-drill) on each Linux and macOS
   runner with the archive it just built: it installs the previous published
   release, upgrades to the new archive, and rolls back;
7. generates checksums and an SBOM;
8. creates an attestation for every published archive;
9. prepares a draft that is not public, from the tag that already exists;
10. downloads the prepared assets on Linux x64/arm64, macOS x64/arm64, and
    Windows x64/arm64;
11. verifies every checksum, GitHub provenance attestation, and Sigstore
    signature;
12. tests each raw binary and the real installer, which fails closed. The
    tests cover schema contracts, the native companion ABI (application
    binary interface), client plans, and production start. The downloaded
    Linux binary also runs the browser/storage and website typing gates
    against the runtime that it emitted;
13. makes the release public only after every verification succeeds.

The workflow never creates or moves a tag. It never overwrites a release
asset. If a verification fails, the draft stays non-public so that people can
investigate.

Cross-building may add to the matrix. It never replaces native evidence for
promotion.

Release qualification always runs on GitHub-hosted runners. `rust-ci.yml`
chooses a self-hosted runner only for pushes to branches when
`TACHYON_SELF_HOSTED` is `true`, and a release tag is not a branch.
[SELF_HOSTED_RUNNERS.md](SELF_HOSTED_RUNNERS.md) describes the self-hosted
runners that maintainers can use for branch CI.

[RECONCILIATION.md](RECONCILIATION.md) lists the local features being
reconciled. Its acceptance gates must pass on the actual downloaded draft
assets, not only on a local developer executable. The existing `v26.35.07`
assets never change. The restored features and the input-focus fix need a new
qualified version.

## Failure Policy

If the gate for one platform fails, downgrade or leave out that platform. Do
not label it wrongly and do not disable the gate. Security fixes for healthy
targets do not have to wait for an unrelated preview platform. However, the
release notes must state the exact reduced matrix.

Never skip a contract, security, upgrade, rollback, packaging, or native
execution test only to keep the release matrix green.

## Rollback

Before a release can be supported:

- installer changes have an uninstall and rollback plan;
- every persistent format has fixtures for forward and backward
  compatibility;
- irreversible migrations are explicit and require a verified backup;
- the previous supported binary can be reinstalled from kept artifacts;
- users can see the rollback limitations before an upgrade begins.

## Lifecycle Drill

`scripts/release/lifecycle-drill.sh` proves that users can upgrade from the
previous release and roll back to it. It runs on macOS and Linux:

1. It downloads the previous release's archive for this machine. It checks the
   archive against the release `SHA256SUMS`, then checks every file inside it.
2. It checks the candidate archive the same way. It also checks that a
   modified binary fails verification, and that the candidate binary is not
   the previous release's binary.
3. It installs the previous `ty` and scaffolds a sample app with it. The app
   has a Yon route that serves `server/data/notes.json`, a `.env` file with
   one `YON_` secret, and a git commit of its source.
4. It bundles and starts the app with the previous `ty`, checks that the app
   serves the stored data, and stops it with SIGTERM. Then it records the
   SHA-256 of every project file outside `dist/` and `.git/`.
5. It upgrades the binary in place, using the same staged rename as
   `install.sh`, and checks that the upgrade changed no project file.
6. It migrates the app the way a user would: `ty migrate check`, then
   `ty migrate fix --write`, with no manual edits. The fix must move the
   route to `server/controllers/` and the page's `tac.css` and `tac.js` out of
   `client/pages/`. Then it bundles and serves the app with the cache and
   `dist/` that the previous release left.
7. It rolls back by downloading the previous archive again and installing its
   binary. It reverts the migration's source changes with `git checkout` and
   `git clean`. Then it bundles and serves the app with the cache and `dist/`
   that the candidate left.
8. It uninstalls and checks that nothing is left behind.

Every installed binary must match its archive byte for byte. The data and
config files (`server/data/notes.json`, `.env` and `package.json`) must be
unchanged after the migration and after each run. After the rollback, every
project file must match what the previous release had.

The drill marks each step with `=== <step> <UTC time>` and
`=== <step> rc=<code>`. It writes `lifecycle-drill.log`, one `start-*.log`
for each server run, `project.sums` and `kept.sums` to `--log-dir`. By
default that is `$LOG_ROOT/TACHYON/lifecycle-drill-<UTC time>`. Without
`$LOG_ROOT`, it uses the runner's or the system's temp folder.

Run it in these three places:

- **Before the release PR:** the release session runs it on the release tip.
  First it builds the candidate with `scripts/release/build-artifact.sh`.

  ```sh
  CARGO_TARGET_DIR=<build folder> scripts/release/build-artifact.sh <out>
  scripts/release/lifecycle-drill.sh --from 26.36.01 --candidate <out>/tachyon-<version>-<arch>-<os>.tar.gz
  ```

- **In `rust-ci.yml`:** the `release-lifecycle` job runs it with no
  arguments. Then the drill builds the candidate twice and checks that the
  two archives are identical.
- **In `publish.yml`:** each Linux and macOS build job runs it on the archive
  it just built, before `stage-release` creates the draft. If a drill fails,
  no draft is created. The job keeps the drill logs as the
  `lifecycle-drill-<asset>` artifact.

`ty start` refuses to serve Yon handlers on a host that cannot confine their
egress (TY2113, ADR 0113), such as Linux without Landlock. When that happens,
the drill logs the refusal and starts the server again with
`YON_EGRESS_ENFORCEMENT=proxy`. Each run logs which enforcement it used.

`--from` defaults to `TAC_DRILL_FROM`, or else to the latest published
release. The drill needs `node` on `PATH` for the sample app's route.
The drill also needs `git` to revert the migration.
`TAC_DRILL_RELEASE_URL` replaces the download folder of the `--from` release.

The drill does not cover Windows; `install.ps1` has its own backup and restore
test in `publish.yml`.

### Rolling Back an Install

To put back a previous release, run the installer with that release's download
folder:

```sh
curl -fsSL https://github.com/d31ma/TACHYON/releases/download/v26.36.01/install.sh \
  | TAC_BASE_URL=https://github.com/d31ma/TACHYON/releases/download/v26.36.01 sh
```

Rolling back the binary does not roll back source changes. If you ran
`ty migrate fix --write` after the upgrade, revert those source changes, for
example with `git checkout`, before you run the older `ty`.
