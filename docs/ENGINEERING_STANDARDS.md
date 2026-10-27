# Tachyon Rust Engineering Standards

## Meaning of Enterprise-Grade

Enterprise-grade means repeatable evidence. It includes:

- explicit ownership;
- stable contracts;
- bounded failure;
- deterministic builds;
- native validation;
- secure defaults;
- recoverable releases;
- documentation that agrees with behavior.

It does not mean the largest number of crates, abstractions, or CI jobs.

Every external build, probe, and hook command must use the shared supervised
runner. Production code must not use any of these:

- direct `Command::output`;
- pipe capture without a size limit;
- a timeout that stops only the leader process;
- detached cleanup of descendant processes.

## Code Style

The reader of every file is an intern who just graduated from college and reads
English as a second language. They must understand every file on the first
read. These rules apply to all languages in the repository.

- Write the simplest code that works.
- Name things with common, literal English words. Do not use idioms, slang,
  puns, rare words, or unexplained abbreviations. For example, write `index`,
  not `idx`, and `buffer`, not `buf`. Single letters are fine only for loop
  counters and very small closures.
- Keep functions short. A function does one thing.
- Do one thing per line. Do not write dense one-liners or long chains that a
  reader must untangle. Give long expressions named intermediate variables.
- Use early returns (guard clauses, `let ... else`) instead of deep nesting.
- Pick the plain construct over the advanced one: a loop instead of a fold, an
  `if` instead of a lookup table of closures. Use the advanced construct only
  when the plain one is wrong. Simple `map`, `filter`, `collect`, and `?` are
  plain constructs.
- Write no comments. Names and structure explain the code. If code needs a
  comment, rename or restructure it instead.
- Keep only the comments that a tool reads:
  - JSDoc type tags such as `@param`, `@type`, `@returns`, and `@typedef`;
  - `@ts-` and eslint directives;
  - shebangs and license headers;
  - the clap `///` docs on the CLI types, because they are the `ty --help`
    output;
  - compiler splice markers such as `//__BEHAVIOR_BODY__`.

### File-Level Variables

Do not declare variables or constants at file (module) level. All variables
are class variables:

| Language | Where the value goes |
| --- | --- |
| Rust | An associated `const` in an `impl` block of the type that uses it. Create a small unit struct only if no type fits. A lazily initialized `static` (`OnceLock`, `LazyLock`) goes inside the function that returns it. |
| JavaScript, TypeScript | A `static` field of a class. |
| Python, Dart, Kotlin, Swift, PHP | A class attribute or static field (`companion object` or `object` in Kotlin, `static let` in Swift). |
| Shell, PowerShell | A variable inside a function where practical. Top-level script settings may stay. |

These things may stay at file level:

- imports;
- type declarations (types, interfaces, enums, structs);
- functions and classes;
- the code that creates the framework's decorators and annotations, for
  example the behavior runtime that the compiler emits.

Framework configuration is data, not code: the Tac configuration is the `tac`
member of `client/manifest.json`
([ADR 0052](adr/0052-the-manifest-is-the-tac-configuration.md)).

Application files with framework annotations and Yon handler files have a
stricter boundary: one class and its imports, with same-type Rust impl blocks
and nested types allowed. Helper declarations move to their own role, model
or shared files. Controllers are entries in `server/controllers/`; route markers are retired.
TestSetup lives beside its test in a separate named file. These are
authored-source rules, not restrictions on compiler-generated composition
([ADR 0044](adr/0044-an-annotated-file-holds-one-class.md)).

## Repository Rules

- Build vertical slices.
- Add a crate only when it has production behavior and tests.
- Keep public contracts under `api/`. Generated output names its source. Never
  edit generated output by hand.
- Record hard-to-reverse decisions in ADRs (architecture decision records).
  Record proposed public behavior in RFCs (requests for comments).
- Do not create empty placeholder directories.
- Do not create catch-all modules named `utils`, `helpers`, `common`, `shared`,
  `misc`, `base`, or `manager`.
- Keep package-local tests beside the code. Keep system-wide tests under
  `tests/`.
- Keep legacy compatibility evidence separate from new internal design.

## Rust Rules

- Use stable Rust only. Pin an exact toolchain version that has all security
  patches.
- The 2024 edition and workspace resolver 3 are mandatory.
- `unsafe` is forbidden by default. A future exception requires all of these:
  - an accepted safety ADR;
  - a small, isolated crate;
  - documented invariants;
  - Miri or sanitizer coverage where applicable;
  - named reviewers.
- Errors are typed and actionable. Never parse human-readable error text.
- Do not panic on untrusted input, configuration, filesystem state, protocol
  messages, or expected runtime failure.
- Constructors validate required invariants.
- Prefer owned immutable values at module boundaries.
- Keep I/O at the boundaries. Keep transformations deterministic.
- Set limits on input size, recursion, queues, concurrency, retries, processes,
  and external reads.
- Do not use mutable global state. See [File-Level Variables](#file-level-variables).
- Do not run application commands through a shell.
- Do not log secrets, full request bodies, capability tokens, or environment
  values that are not redacted.
- Avoid feature flags that create unsupported combinations. Every supported
  combination must compile and pass tests in CI.
- The `fuzzing` feature of `tachyon-core` exposes crate-private parsers and
  decoders to the fuzz targets. Only the fuzz crate enables it. Never enable
  it in a released artifact.

## Yon Boundaries

- A controller entry lives in `server/controllers/` and declares one
  `RequestRoute` class. Workers retain their separate Handler protocol.
- Managed providers may infer constructor tokens from required fields with
  NeedsAll. Authored `Needs` lists and manual constructors remain
  optional alternatives (ADR 0025).
- Discovery validates the graph and lowers it to the existing Handler ABI
  (application binary interface).
- Controller endpoints use `GetRoute`, `PostRoute`, `PutRoute`, `PatchRoute`
  or `DeleteRoute`. Their names start with the matching verb. Parameters and
  model return types define validation and the generated schema. Helpers do
  not register endpoints. Annotations do not run during discovery. See ADR 0048.
- Routes, middleware, and workers use the framework-owned Yon languages.
  Other programs must cross an explicit `relay(command, request)` boundary or
  a `@Gateway` command. Project files cannot register an interpreter or select
  an executable handler.
- Relay, Gateway and streaming implementations must:
  - drain bounded pipes concurrently;
  - use one absolute deadline for each request. The supervisor writes the
    time left into the request at the frame write; a runner measures it from
    its own start (a warm runner, from the moment it reads the request
    frame) and never reads it back from a request that the handler can
    change;
  - start outside programs with only the baseline environment and the
    explicit allowlist (a relay program also keeps `YON_HANDLER_PROTOCOL`
    and its handler's `NO_PROXY` values), never `YON_APPLICATION_JSON` or a
    `YON_RESOURCE_*` value, and never with a descriptor above 2 that the
    handler opened. Treat this as protection against accidental inheritance,
    not as a sandbox;
  - connect to the resource endpoint directly, never through a proxy;
  - redact child diagnostics from public responses;
  - prove that they reap descendant processes on timeout and when the
    subscriber closes.
- A Gateway call crosses the per-invocation resource endpoint with a key that
  the build declared. It never gets a shell, never puts arguments on a command
  line, allows one hop only, and reports failures as one of four fixed kinds
  (ADR 0042).
- Environment variables use `TAC_` or `YON_`. `TACHYON_*` names are forbidden.
- Topic SSE (server-sent events) clients must:
  - handle the named `topic-error` event;
  - parse its canonical JSON payload;
  - close on `terminal: true`.
- When the cursor is stale, the client recreates the subscription without an
  explicit cursor. HTTP 503 admission failures use bounded backoff.
- Topic limits:

  | Limit | Value |
  | --- | --- |
  | Global subscribers | 128 |
  | Subscribers per topic | 32 |
  | Active topics | 32 |
  | Replay records | 256 |
  | Size of one record | 64 KiB |
  | Size of one log | 16 MiB |

## Handler Cache Operations

Application behavior caches are separate from the compiled-artifact caches
described later in this section.

- Tac/Yon annotations that are shared across supported languages use explicit
  adapters.
- Contexts require partitioning and cancellation.
- Cache access requires an authorization hook.
- Browser code never receives server credentials.
- A language accepts no behavior marker until its execution adapter is
  implemented and tested. See ADR 0022 and `BEHAVIOR_ANNOTATIONS.md`.

Rules for managed resources:

- Yon may supply managed datasource and Redis adapters through its private
  resource capability (ADR 0027).
- Configuration never grants application authorization.
- Keep invocation tokens out of client artifacts, public responses, and logs.
- Connections belong to the server lifecycle, not to reusable handler
  processes.
- Set separate limits for SQL parameters, identifiers, results, and operation
  lifetimes.
- Never imply schema migration, transaction rollback, or automatic retry of
  mutations.

Compiled and staged Yon artifacts live in the project-local
`.tachyon/handlers` directory.

- After Tachyon opens that directory, every lookup and change is relative to
  its directory handle. The handle does not follow symbolic links.
- The cache is content-addressed: each entry is named by a digest of its
  content.
- Pruning keeps the cache at no more than 256 entries and 512 MiB. The count
  includes nested entries.
- A Kotlin, Rust, Dart, Go, Groovy or C++ artifact is also copied to
  `runtime/handlers-v1` in the cache that `TAC_CACHE_DIR` selects, under the
  same digest name. When the project cache has no valid artifact, Tachyon
  restores a regular, non-empty shared copy instead of compiling again. It
  never follows a symbolic link there. The shared folder keeps the newest 512
  entries, and a shared-folder failure never fails a build
  ([ADR 0037 amendment](adr/0037-csharp-services-reach-the-web-as-webassembly.md)).

Tachyon normally recovers abandoned build locks automatically:

- If the process that holds a lock is gone, Tachyon reclaims the lock
  immediately.
- If another process reused the process identifier, a hard ten-minute lease
  limits how long the lock lasts.

Cache pruning then accounts for and removes the abandoned digest and the
temporary publication files. It does this in the same capability-confined prune
transaction.

C# handler builds turn off reuse of the persistent .NET build server. The
supervised command must own compilation. Compilation must not depend on a
shared compiler server that may keep running after its toolchain volume is
gone. This setting does not turn off Tachyon's content-addressed artifact
cache, and it does not increase compiler deadlines.

To inspect an incident, list `.tachyon/handlers` without following symlinks.
Keep the directory for diagnosis. Never delete a single `*.lock` file or
artifact while `ty serve`, `ty bundle`, or another Tachyon build may still be
running. For manual recovery:

1. Stop every Tachyon server and build process that uses the project.
2. Rename `.tachyon/handlers` to a diagnostic backup on the same filesystem.
3. Run the original Tachyon command. It creates a new confined cache and
   rebuilds the content-addressed artifacts.
4. Remove the backup only after the rebuild succeeds and you have collected
   any incident evidence you need.

`ty cache status` and `ty cache clean` work on the installation/runtime cache
that `TAC_CACHE_DIR` selects. `ty cache clean` removes the shared handler
copies in `runtime/handlers-v1`. Neither command inspects, repairs, or removes
the project-local `.tachyon/handlers` cache.

## Naming

- Name each crate and module for one responsibility.
- Domain names come from `CONTEXT.md`.
- Use common, literal English words. See [Code Style](#code-style).
- Project-owned environment variables start with:
  - `TAC_` for compiler, client, packaging, and repository tooling;
  - `YON_` for server and handler runtime policy.

  Do not add a `TACHYON_` environment namespace.
- Avoid `Impl`, `Util`, `Helper`, `Manager`, and `Common`.
- Application role annotations require that the type name matches the file
  name and that the parent directory name is plural. Behavioral modifiers do
  not change where a file goes.
- Each realm has shared role folders: `client/<role>` and `server/<role>`.
  Controllers are the exception: each controller sits alone beside its route
  ([ADR 0031](adr/0031-realms-mirror-each-other.md)).
- Discover annotated roles of the same language only in those folders. Use the
  captured snapshot and a bounded traversal that does not follow links.
- Nested entries stay separate. The client and server realms stay separate.
- Authored module manifests fail with migration guidance. See
  [ADR 0027](adr/0027-scoped-discovery-and-managed-resources.md).
- Machine fields use `snake_case`.
- Diagnostic codes use `TY` plus four digits.
- Identifiers are opaque. Consumers do not infer security or type from the
  shape of an identifier.

## Contract Rules

Application Configuration v2:

- It uses only the tac/yon namespaces.
- Configuration JSON holds no secrets. Application secrets are literal `YON_`
  entries in the `.env` file at the project root.
- Underscores map to nested paths.
- Client artifacts contain only the base tac projection. They never read
  `.env`.
- Preserve the private snapshot lifetime, redacted diagnostics, source
  confinement, and separate deployment authority.
- Ordinary Yon results are serialized once. Optional response helpers own the
  HTTP metadata.
- A JSON parse failure must not silently become an empty object.

General contract rules:

- Schemas, fixtures, error codes, and artifact manifests are canonical.
- Prefer additive changes within a major version.
- A breaking change requires all of these:
  - an ADR or RFC;
  - a migration;
  - a deprecation window;
  - compatibility tests;
  - a new major version.
- Behavior for unknown fields and unknown enum values is explicit.
- Mutating operations document idempotency and retry safety.
- Human-readable messages are not parsing contracts.

## Test Standards

- Unit tests prove deterministic transformations and invariants.
- Table, property, and model tests cover state spaces.
- Golden tests cover stable diagnostics and generated artifacts.
- Contract tests run against real compiled processes or artifacts.
- Fuzz tests cover every parser and every external protocol boundary.
- Adversarial tests cover inputs that are malformed, oversized, replayed,
  concurrent, path-escaping, resource-exhausting, or capability-escalating.
- A native claim requires execution on the native OS and architecture.
- Tests are deterministic, except fuzz, chaos, load, and soak tests that are
  marked as such.
- Whole-workspace coverage must not fall below 80% for lines, functions, or
  regions.
  - The measurement includes CLI, operating-system, toolchain, and fuzz-only
    code.
  - Only the external native compiler and the macOS host glue code named by
    ADR 0012 are excluded.
  - High-risk parser, path, compiler, and server boundaries need focused
    adversarial and end-to-end tests, whatever the total coverage is.
- A CRAP (change risk anti-patterns) score of 15 is the absolute maximum for
  one function. It is not the target.

## Review Expectations

Every material change states:

- the invariant it protects;
- observable acceptance criteria;
- compatibility consequences;
- failure modes and recovery;
- threat-model changes;
- validation evidence.

Do not mix generated changes with unrelated handwritten refactors. Do not ship
debugging output, sample secrets, stale fixtures, or TODOs without an owner.
