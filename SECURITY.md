# Security Policy

## Reporting a Vulnerability

Use GitHub's private vulnerability reporting for the `d31ma/Tachyon`
repository. Do not open a public issue, discussion or pull request that
contains any of these:

- a vulnerability that is not fixed yet;
- an exploit;
- a secret;
- private user data.

Include in your report:

- the affected commit or release;
- the affected target and configuration;
- the smallest reproduction or proof;
- the impact you expect;
- any workarounds you know;
- whether you think someone is already exploiting it.

When a report is complete, the maintainers will:

1. confirm that they received it;
2. set up a private channel to work with you;
3. find out which versions are affected;
4. tell you their plan to fix and disclose the problem.

The maintainers will publish exact response times before Tachyon calls a Rust
release supported.

## Supported Versions

The Rust rewrite has finished engineering phases 0–7. Version `26.35.07` is
still a release candidate. It has no supported tier and no preview tier. The
public release notes list the supported released versions of the existing
implementation. `docs/SUPPORT_TIERS.md` lists the evidence that a Rust target
needs before it can move to a higher tier.

## Security Update Policy

- Critical fixes come before feature work.
- The project never replaces a release artifact without telling anyone.
- A corrected release gets a new version that never changes, and new
  provenance (a record of how it was built).
- The project can remove or lower the tier of a vulnerable target on its own,
  without changing other targets.
- The project documents a workaround only if it fails safely (it fails closed)
  and does not create a larger vulnerability.

## Scope

The first threat model is `docs/THREAT_MODEL.md`. These areas are especially
sensitive:

- parsers;
- keeping file access inside allowed folders (filesystem containment);
- template escaping;
- handler supervision;
- capability bridges;
- WebSurface isolation;
- installers;
- updater logic;
- CI (continuous integration);
- signing.
