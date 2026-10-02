# ADR0091: Role folders allow nesting and require explicit registration

- Status: Accepted by Samuel,2026-10-01; qualification in progress
- Amends ADR0031 shared realm folders and ADR0044 behavior-only services
- Preserves ADR0047 marker names, ADR0048 annotation routes, ADR0064 annotation-free utilities and ADR0088 remotes compatibility
- Preserves ADR0043's data-only models boundary; the approved declarative metadata adapters are under qualification

## Decision

The shared client/server role folder remains the discovery root. Authors may organize files under nested directories within that root. Nested services and controllers discover collaborators across the whole realm, using actual relative paths when composing source, rebasing imports and hashing services. Organization does not create a new injection namespace. Duplicate service/language variants remain errors. Discovery uses existing capability-based no-symlink opens, deterministic order,4096-entry and32-level limits and existing entry boundaries.

A concrete authored role class/struct/object/record beneath services, stores, remotes/legacy clients, setups, entities, supported helpers or converters requires that folder's existing marker. Server controllers require RequestRoute, SocketRoute, ErrorHandler or IdentityProvider. Marker syntax and availability remain language-specific. A missing marker fails with TY1363, naming the path and required spelling. Correct placement, class suffix, single-class and role-specific argument checks continue to apply.

This explicitly supersedes the former ADR0044 allowance for a behavior-only Tac service without Service. There is no compatibility bypass enabling silent registration of concrete unmarked code. Existing apps must review migration findings, add the marker only to a genuine registered role, or relocate the collaborator to its proper plain-source folder.

Abstract/interface/protocol/trait contracts, type aliases, export-only barrels and assets are not implicitly registered roles. Compiler-generated output is outside authored role capture. Tests outside application discovery remain fixtures. Models, plain shared code, native errors and annotation-free static utils retain their dedicated contracts. Helpers are formal roles. A *Service declaration beneath helpers fails suffix/folder consistency; it belongs beneath services with Service. Missing compiled-language Helper support is an explicit implementation prerequisite. Until qualified, unsupported Helper use fails clearly; helpers cannot act as an unchecked dumping ground.

`ty migrate check` reports required role.marker changes. It does not blindly add markers, construct Store arguments, choose a controller route, or grant helper code an injectable lifecycle. Authors review the intended role and move misplaced collaborators. Legacy clients remains accepted; retired folders retain their existing migration errors.

## Migrating declarative data holders

Samuel approved moving configuration and validation data holders out of helpers and into the realm's models folder, with the missing Yon metadata adapters implemented before runtime qualification. Existing `Settings`, field `Setting`, `Checked` and supported field constraints describe data; they do not register a Helper or Service. Capture that metadata before removing framework annotations for the target compiler, and retain the language's ordinary model namespace and imports.

This approval includes the audited JavaScript/TypeScript `ApplicationSettings`, `ApplicationValues` and `ProductInput` data holders. It extends model handling for those declarative shapes; it does not allow arbitrary JavaScript behavior classes in models. A type with state-changing methods, configuration factories, adapter operations or other behavior remains outside the data-only boundary of ADR0043. Existing language-specific data constructors remain subject to that ADR. No general constructor or method exemption is introduced; the C# no-instantiation conformance case needs its separately qualified adapter evidence.

Classify each migration by its actual code:

| Authored declaration | Intended destination |
| --- | --- |
| `ApplicationValues` fields with `Setting` metadata | models, with captured configuration metadata |
| `ApplicationSettings`/`Settings` fields and constraints | models, with captured settings metadata |
| `ProductInput`/`Input` fields with `Checked` and constraints | models, with captured validation metadata |
| Stateful `InputService` with `calls` and a `Check`-guarded `submit` method | services, with `Service` |
| A genuine helper collaborator with behavioral methods | helpers, with `Helper` and the existing helper naming rules |

Keep data type names and consumer references stable when moving models. Add explicit language imports where model projection requires them. Do not add fake Helper markers to data holders or rename a Service into a Helper. Historical neutral and release fixtures remain immutable; compatibility tests migrate generated copies with explicit path and role mappings.

The model metadata adapters and affected runtime fixtures are under qualification. The approved migration policy is not evidence that every language's configuration and validation matrix passes.

## Qualification

Initial recursive discovery24tests pass, including nested Tac DI paths, both-realm nested providers and nested imports. Final guard/language, real compilation/runtime, safety and combined compiler evidence will be recorded at the qualified checkpoint. No deployment or architecture refactor is implied.

## Native Rust dependency qualification boundary

Native Rust service composition includes same-language peer services in the dependent owner namespace, preserving required constructor metadata while consuming role markers internally. Authored captures are validated before that consumption. Ordinary script/browser sibling isolation remains unchanged. The exact monitor Arc dependency is the immediate construction test. Cross-root singleton identity and dependency lifecycle hooks require a shared native provider graph; local owner construction does not prove those semantics. Trait implementation selection and non-wire native internals need explicit typed bindings, not inferred implementations.

## Caller-owned C# Tac Helper boundary

The missing C# Tac Helper prerequisite is implemented first as explicit same-language caller-owned composition, with qualification pending. A genuine marked Helper retains its typed constructor and must have a real `new` call outside its own class. Setup or Service callers retain ownership of any captured context. Class-level Helper provider/lifetime declarations, inferred managed Helper dependencies and Helper lifecycle/event hooks remain explicitly unsupported and fail before authored metadata is consumed. Field `Needs` constructor lowering is available only for explicitly called typed constructors. This scope does not establish recursive managed C# Helper DI or automatic cleanup. See [prepared qualification evidence](../evidence/TAC-CSHARP-CALLER-OWNED-HELPERS.md).
