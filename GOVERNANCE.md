# Tachyon Governance

## Principles

Tachyon makes decisions based on public technical evidence and documented
decisions. People become maintainers in proportion to how much they contribute
over time. No title is more important than a contract, a test, a security gate
or a release requirement.

## Roles

- **Contributors** suggest and implement changes.
- **Maintainers** review changes and own subsystems. They keep CI (continuous
  integration) and the documentation in good condition.
- **Release maintainers** control the protected release environments, the
  signing identities and the promotion of platforms to higher support tiers.
- **Security maintainers** receive private reports and coordinate disclosure.

One person can have more than one role. When there are enough contributors,
someone other than the author should review release approvals and
security-sensitive changes.

## Decision Process

- Normal changes that are easy to undo use pull-request review.
- Internal decisions that are hard to undo use ADRs (architecture decision
  records).
- New public behavior, or public behavior that breaks compatibility, uses RFCs
  (requests for comments).
- A change to security-sensitive behavior also updates the threat model.
- A platform can move to a higher tier only with the evidence in
  `docs/SUPPORT_TIERS.md`.

When the maintainers cannot agree, they write down:

- the alternatives;
- the evidence;
- who owns the decision;
- how to appeal it.

Authority that is silent or not documented cannot make a decision.

## Compatibility

A change to stable behavior needs all of these:

- a documented migration;
- compatibility fixtures;
- a suitable deprecation period.

A security fix can shorten that period when keeping compatibility would keep
a vulnerability. The release notes must explain the exception.

## Maintainer Changes

A person becomes a maintainer after contributing good work and reliable
reviews over a long time. An inactive maintainer can move to emeritus status
(an honorary former maintainer) and keep the credit for their work. When a
maintainer leaves, or their account is compromised, their access is removed
quickly. Access is also reviewed regularly, and each person gets only the
access they need (least privilege).
