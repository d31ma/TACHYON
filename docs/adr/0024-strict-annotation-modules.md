# ADR 0024: Strict Role Names and Explicit Annotation Modules

- Status: Accepted
- Module selection is superseded by ADR 0027: annotation-driven entry scopes
  replace authored manifests. The naming rules below remain valid; the
  placement rule is amended by [ADR 0031](0031-realms-mirror-each-other.md),
  which shares role folders per realm and leaves only controllers beside a
  route.
- Date: 2026-09-12
- Amends: ADR 0021 annotation discovery and ADR 0022 source-local declarations
- Preserves: ADR 0015 client-only Tac, ADR 0016 REST-only Yon, ADR 0023 private configuration
- Amended by: [ADR 0046](0046-tachyon-annotations-are-global.md): the compiler-owned `tachyon:behaviors` import is refused; annotations and types are global

- Follow-up: [ADR 0025](0025-required-constructor-inference.md) infers managed Yon dependencies without changing role layout or module selection

## Context

Role declarations inside the reserved tac.* and yon.* filenames cannot follow
an enforced filename convention for domain roles. Applications need
predictable names and locations in all supported languages. They must not
need reflection scanning, implicit registration or a global container.
Spring-style organization inspired this design. Spring Boot does not require
Tachyon's exact naming rules.

## Decision

Each role type needs a name suffix and a plural immediate parent folder:

| Role suffix   | Parent folder  |
| ------------- | -------------- |
| Controller    | controllers    |
| Service       | services       |
| Repository    | repositories   |
| Client        | clients        |
| Component     | delegates      |
| Configuration | configurations |
| Entity        | entities       |

Tachyon also enforces these rules:

- The name needs a domain prefix.
- The filename stem must equal the type name, with the exact case.
- One file has one primary role type.
- Rust impls are associated with their target type.
- Conflicting roles are rejected.

Modifiers such as Cache, Table, RequiredArgsConstructor,
ConfigurationProperties, Profile and Value add no naming rule. Existing
framework and language role support stays unchanged.

A sibling yon.modules.json or tac.modules.json selects modules explicitly.
Each key is a language extension. It holds an ordered list of
project-relative paths for that language. The immutable project snapshot
captures the manifest and the sources. Tachyon validates the original paths
before lowering. It assembles the entrypoint and the selected modules into
one source-local compilation unit. Tachyon does not:

- scan folders;
- follow imports recursively;
- mix languages;
- implicitly share metadata between independent generated units.

Role declarations move out of reserved entrypoint files. Ordinary Handler
entrypoints stay valid. A Yon entrypoint that contains only comments may get
verb discovery from a listed controller. Native Tac keeps its contract: its
selected page type comes first, and it is constructed with no arguments.
These stay unchanged: context creation, explicit tokens, scope boundaries,
adapter authority and private configuration.

At first, included modules reject ordinary relative imports. After assembly,
the base path of a relative import would be ambiguous. These imports stay
allowed:

- the compiler-owned tachyon:behaviors;
- supported nonrelative language/standard-library imports.

This is not a general cross-language linker or package resolver.

## Consequences

- Wrong suffixes, wrong case, wrong immediate folders and conflicting roles
  fail discovery or build. They do not silently differ between developer
  machines.
- Grouping by feature stays flexible above the immediate role folder.
- Migration renames inline role types, updates references and adds
  manifests. Types that have only modifiers do not need to be renamed or
  moved.
- Explicit manifests make cross-file ownership deterministic and easy to
  review. The cost is that someone must maintain a selection list.
- Folders grant no environment, network, database or DI (dependency
  injection) authority. No private application configuration crosses
  automatically into Tac.

## Acceptance

Tests cover:

- names, case and plural folders;
- multiple and conflicting roles;
- Rust targets;
- declarations that have only modifiers;
- unsafe or malformed manifests;
- language and realm boundaries;
- missing and duplicate selections;
- snapshot reuse;
- rejection of unsupported relative imports.

Integration tests exercise Yon dispatch and Tac build paths. The website shows
eight Yon languages and the six supported Tac companion languages. It states
the native target limits explicitly.
