# ADR 0125: Store Tables Load by Partition

- Status: Accepted
- Date: 2026-11-11
- Amends: [ADR 0059](0059-tac-stores-use-indexeddb-and-caches-use-the-cache-api.md)
  (item 1: the 10,000-row limit and the indexes; item 4: what a WebAssembly
  or Dart service copies into memory before it starts)
- Preserves: [ADR 0065](0065-webassembly-services-reach-the-browser-through-tachyon.md)
  (the synchronous page bridge and the caller rule for later answers) and
  [ADR 0084](0084-webassembly-services-stay-on-the-page-thread-until-dotnet-needs-a-worker.md)
  (its storage proxy is the place where this also works in a worker)

## Context

Mesail, a messaging application on Tachyon, keeps every message in one Tac
Store table, and wants months of messages on the device (CADUCEUS ADR 0006,
request T1). ADR 0059 gave two limits that stop this:

1. Before a WebAssembly or Dart service starts, the page copies **every**
   table of the `tachyon-store` database into memory, even tables the page
   never reads.
2. A table above 10,000 rows is marked as too large, and every use of it fails.
   The JavaScript adapter reads at most 10,000 rows for one query.

A WebAssembly service reads its Store synchronously (ADR 0065), so the rows
it reads must already be in memory. The answer is to keep in memory only the
part of a table that the page needs now: one **partition**, the rows that share
one value of a field, such as one conversation.

## Decision

1. **Declaration.** An Entity names its partition field in its existing
   options: `@Entity({"adapter":"indexeddb","partition":"conversationId"})`
   (in every language's marker form). The value is a field name. Only the
   `"indexeddb"` adapter accepts it; another adapter fails the build
   (`TY1308`).
2. **Every query names one partition.** On a partitioned Entity, a derived
   query or `@Query` must compare the partition field with Equals, for example
   `findByConversationIdAndSentAfter`. Otherwise the build fails with
   `TY1308`, as every other Store rule does. Such a Store cannot extend
   `Repository`, because the base's `findById` and `findAll` name no
   partition. The resolved query keeps `"partition"` in its entity, and every
   entity command of a partitioned Entity carries `"partition": "<field>"`.
3. **Storage.** The object store gets one index named `tachyon:partition` on
   the partition field. The schema upgrade adds it (and replaces it when the
   field changes). An index is not data: no row is deleted. A table that was
   stored by partition and whose Entity no longer names one stops with a
   message, as a changed Id field does.
4. **The 10,000-row limit is per partition.** A table may hold any number of
   rows. The JavaScript and TypeScript adapter reads a query through the
   partition index first, so it reads at most one partition. It needs no
   `load`.
5. **WebAssembly and Dart services load a partition on demand.**
   `context.repository(Entity)` gains `load(partition)`, `loaded(partition)`
   and `unload(partition)` in Rust, Go, C#, F#, Swift, Kotlin, Java, Dart,
   Ruby, Lua, Python and C++ (each in its own spelling, such as Go's `Load`).
   - `load` answers at once. The page reads the partition from IndexedDB
     after the writes that are already waiting, then sends
     `StorePartitionLoaded { table, partition, rows }` to the caller (the
     caller rule of ADR 0065). Writes made while it loads are applied on top,
     so nothing is lost. `load` of a loaded partition reads it again.
   - A partition above 10,000 rows, or a failed read, sends
     `StorePartitionFailed { table, partition, reason }` instead.
   - `loaded` answers true or false at once. `unload` drops the rows from
     memory; writes that are waiting still finish.
   - A read of a partition that is not loaded fails with the error code
     `storage_not_loaded` and a message that names the partition and `load`.
     It never answers "empty". A repository `find` (no partition) on a
     partitioned table fails with a message.
   - A write to a partition that is not loaded is allowed. It goes to
     IndexedDB in order and does not load the partition.
   - `partition` in the events is text (a number partition is sent as its
     text); `rows` is a number.
6. **Start-up copies only unpartitioned tables.** Before a WebAssembly or Dart
   module starts, the page skips every object store that has the
   `tachyon:partition` index. Unpartitioned tables are still copied as in
   ADR 0059, so a large unpartitioned table still fails as before.
7. **A failed background write is not silent.** When a write of a WebAssembly
   or Dart service fails in IndexedDB (for example `QuotaExceededError`), the
   page sends `StoreWriteFailed { table, reason }` to the service that wrote,
   or publishes it when no service call was running. The memory copy keeps
   the write, so the copy and the database can differ until the page reloads.
8. **The three events** are built-in event models (one table makes their types
   in every language and in `types/tachyon-behaviors.d.ts`).

### Browsers and native shells

IndexedDB indexes and `getAll(query, count)` work in Chromium, Firefox,
Safari 14 and newer and every web view of the native shells, so no shell
needs new code. A natively linked companion has no page; it keeps its Setup
adapters (ADR 0059 item 4), and because a partition needs the
`"indexeddb"` adapter, it cannot declare one.

### Security

There is no new trust boundary: the rows stay in the origin's IndexedDB. The
partition value is a parameter of an IndexedDB key range, never text built
into a query. A partition value must be a string or a finite number.

## Consequences

- Mesail keeps the conversation list as an eager table and its messages in a
  table partitioned by `conversationId`. Opening a conversation calls `load`
  and renders a "loading" state until `StorePartitionLoaded` arrives; the
  memory budget code calls `unload`.
- Memory still grows with each loaded partition. An application must
  `unload` what it no longer shows.
- A delete query (`deleteBy...`) on a partition that is not loaded fails with
  `storage_not_loaded`, like a read, because the page cannot count the rows
  synchronously. A partition above 10,000 rows can be cleaned from a
  JavaScript or TypeScript service, which reads through the index.
- Start-up still copies every unpartitioned table of the database, also tables
  that the page's services do not use. Copying only the page's tables needs a
  list of tables per page in the build output; it is left for later, because
  the large tables are the partitioned ones.
- A partitioned table cannot extend `Repository`; services save and delete
  through `context.repository(Entity)` or derived `deleteBy` queries that name
  the partition.
- Later slices of the offline-first plan (sync between tabs, sealed tables,
  worker services) change the same page code (`TacEntityMirror` and
  `TacEntityPartitions` in `tac-storage.js`) and build on these calls.
