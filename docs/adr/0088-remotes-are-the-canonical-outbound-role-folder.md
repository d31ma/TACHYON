# ADR 0088: Remotes are the canonical outbound role folder

- Status: Accepted by Samuel; scoped implementation and qualification complete
- Date: 2026-09-30
- Relates to ADR0031 (shared realm roles), ADR0047 (markers) and ADR0087 (injected HTTP)

## Decision

Use `client/remotes` and `server/remotes` for existing outbound HTTP, socket and
gateway roles. Keep the top-level client and server realms and the existing
Client, HTTPClient, SocketRequest and Gateway marker contracts. This changes
folder naming, without introducing an additional container or role API.

The previous `client/clients` and `server/clients` paths remain compatibility
aliases. Discovery, source capture, role composition and provider hashes accept
both spellings. Duplicate class identities across the aliases are diagnosed;
there is no order-dependent provider selection.

New scaffolds, maintained documentation and examples use remotes. Historical
ADRs and dedicated clients compatibility fixtures remain historical evidence.
The earlier integrations candidate was superseded by the explicit remotes
choice and is not a second canonical alias.

## Migration

Existing apps need no forced move. The opt-in migrate fix plan moves a legacy
folder and rewrites recognized imports only when its destination is safe.
Destination collisions are refused. Custom or external imports require explicit
review; migration does not invent paths or change marker names.

## Evidence and limits

The scoped implementation is qualified at source checkpoint389b5319, including
both-realm alias/duplicate checks, safe migration/collision tests, scaffolds,
published HTTP/gateway requests and focused JS/TS HTTP/socket browser probes.
See [the queue brief](../queue/NEXT/REMOTES-FOLDER.md) for retained evidence.
That qualification does not certify all language/platform combinations or
complete the separate browser transport and native page-program workstreams.
