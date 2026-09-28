# ADR 0005: Versioning, Compatibility, and Support Evidence

- Status: Accepted
- Date: 2026-07-26

## Context

The rewrite must:

- let machine contracts change over time;
- keep the current developer workflow where that is intended;
- later replace a released implementation.

It must do this without treating build output as proof of platform support.

Tachyon already uses a UTC CalVer (calendar versioning) product version. Cargo
packages need package metadata that is compatible with SemVer (semantic
versioning). Public schemas need their own major versions.

## Decision

- Keep Tachyon's UTC CalVer identity for product releases.
- Use internal Cargo package versions only as implementation metadata.
- Record these values in Artifact Manifest v1: the exact product version, the
  commit, the toolchains, the contracts, the target, the inputs, the outputs
  and the digests.
- Give each public machine contract its own version, stored in a directory per
  major version.
- Within one major contract version, prefer changes that only add things.
- A breaking contract change requires an RFC (request for comments), a
  migration, compatibility tests and a deprecation window.
- Keep the current public behavior with neutral compatibility fixtures. Do not
  import legacy internals for this.
- Use the support vocabulary in `docs/SUPPORT_TIERS.md`.
- Never promote a platform only because cross-compilation works.

During Phase 0, the rewrite version is `0.0.0-phase0`. Tachyon must not publish
this version as a preview or stable release.

## Consequences

- Product versions and internal package versions can change separately, and
  nobody mixes them up.
- Each artifact describes itself, and anyone can verify it on its own.
- Marketing claims about a platform need evidence. Release engineering owns
  that evidence.
- Every difference from the legacy implementation must be intentional,
  documented and testable.
- Contract consumers can negotiate compatibility without parsing text written
  for humans.

## Rejected Alternatives

- **One version for every schema and package**: unrelated compatibility
  changes would have to happen at the same time.
- **Semantic versioning for the public product immediately**: it would change
  the current Tachyon release identity for no need.
- **Support on successful cross-build**: a cross-build gives no evidence that
  the native runtime works.
- **Big-bang replacement without dual-run fixtures**: replacing everything at
  once, without fixtures that run on both implementations, hides regressions
  until cutover.

## Acceptance Gate

- Artifact Manifest v1 records the required provenance fields;
- the release and support documents use the same vocabulary;
- the rewrite documentation clearly separates the legacy baseline, preview and
  stable cutover;
- no Phase 0 workflow publishes a Tachyon release.
