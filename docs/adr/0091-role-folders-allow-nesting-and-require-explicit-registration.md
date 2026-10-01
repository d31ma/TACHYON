# ADR0091: Role folders allow nesting and require explicit registration

- Status: Accepted by Samuel,2026-10-01; qualification in progress
- Amends ADR0031 shared realm folders and ADR0044 behavior-only services
- Preserves ADR0047 marker names, ADR0048 annotation routes, ADR0064 annotation-free utilities and ADR0088 remotes compatibility

## Decision

The shared client/server role folder remains the discovery root. Authors may organize files under nested directories within that root. Nested services and controllers discover collaborators across the whole realm, using actual relative paths when composing source, rebasing imports and hashing services. Organization does not create a new injection namespace. Duplicate service/language variants remain errors. Discovery uses existing capability-based no-symlink opens, deterministic order,4096-entry and32-level limits and existing entry boundaries.

A concrete authored role class/struct/object/record beneath services, stores, remotes/legacy clients, setups, entities, supported helpers or converters requires that folder's existing marker. Server controllers require RequestRoute, SocketRoute, ErrorHandler or IdentityProvider. Marker syntax and availability remain language-specific. A missing marker fails with TY1363, naming the path and required spelling. Correct placement, class suffix, single-class and role-specific argument checks continue to apply.

This explicitly supersedes the former ADR0044 allowance for a behavior-only Tac service without Service. There is no compatibility bypass enabling silent registration of concrete unmarked code. Existing apps must review migration findings, add the marker only to a genuine registered role, or relocate the collaborator to its proper plain-source folder.

Abstract/interface/protocol/trait contracts, type aliases, export-only barrels and assets are not implicitly registered roles. Compiler-generated output is outside authored role capture. Tests outside application discovery remain fixtures. Models, plain shared code, native errors and annotation-free static utils retain their dedicated contracts. Helpers are formal roles. A *Service declaration beneath helpers fails suffix/folder consistency; it belongs beneath services with Service. Missing compiled-language Helper support is an explicit implementation prerequisite. Until qualified, unsupported Helper use fails clearly; helpers cannot act as an unchecked dumping ground.

`ty migrate check` reports required role.marker changes. It does not blindly add markers, construct Store arguments, choose a controller route, or grant helper code an injectable lifecycle. Authors review the intended role and move misplaced collaborators. Legacy clients remains accepted; retired folders retain their existing migration errors.

## Qualification

Initial recursive discovery24tests pass, including nested Tac DI paths, both-realm nested providers and nested imports. Final guard/language, real compilation/runtime, safety and combined compiler evidence will be recorded at the qualified checkpoint. No deployment or architecture refactor is implied.
