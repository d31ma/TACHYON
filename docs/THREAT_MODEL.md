# Tachyon Rust Rewrite Threat Model

Tachyon treats toolchains and project hooks as untrusted child process trees.
For each of them, Tachyon:

- limits the wall-clock time and the amount of output it keeps;
- ends the whole POSIX process group or Windows Job Object on completion,
  cancellation or timeout.

Topic log subscriptions have these controls:

- They accept only bounded slugs (short, URL-safe names).
- They reject inputs that are symlinks, that are not regular files, or whose
  identity changes during the check (a race).
- They read at most 16 MiB in total.
- They remove subscribers that fall behind or go over capacity, without
  showing filesystem details.
- Topic errors use a named `topic-error` SSE (server-sent events) event with
  canonical, sanitized JSON. They never reuse the handler-stream
  `error`/`TY2107` contract, and they never include raw log content.

## Scope

This model covers:

- project discovery;
- HTML and companion parsing;
- compiler inputs and outputs;
- route manifests;
- handler processes;
- development and production servers;
- generated web artifacts;
- native web-view hosts;
- capability bridges;
- caches;
- installers;
- update channels;
- release artifacts;
- CI.

This model is updated continuously. Every vertical slice (each end-to-end
feature) updates its concrete data flows, trust boundaries, abuse cases and
validation evidence.

### Yon annotation frontend

Controller and provider markers are untrusted source syntax. They never give
authority:

- `Service`, `Repository`, `Client` and `Helper` do not grant network,
  filesystem or process capabilities.
- The bounded frontend produces direct constructor calls. It does not evaluate
  annotation expressions, and it does not import application code.
- It rejects unresolved tokens, cycles, capture of a request-lifetime value by
  a singleton, duplicate registration and unsupported declaration shapes.

Limits:

| Item | Limit |
| --- | --- |
| Source size | 1 MiB |
| Providers | 128 |
| Dependency edges per provider | 32 |
| Identifier length | 128 bytes |
| Instance constructions per invocation graph | 4,096 |

The last limit stops transient providers from growing exponentially before any
provider runs.

Generated entries for interpreted languages:

- Each generated entry has a digest-based name and sits next to the authored
  file inside the project snapshot that Tachyon owns. So captured relative
  imports still work.
- Tachyon does not overwrite captured authored files.
- When annotations are present, both process dispatch and Firecracker dispatch
  use the generated entry. Neither opens the authored project path again.
- Discovery captures streaming metadata once. Tachyon does not parse it again
  from changeable source on each request.

New parser and fuzz tests, and real dependency injection (DI) tests for all
eight adapters, cover this boundary. These guarantees do not make an
operator-owned Firecracker driver qualify as production isolation.

## Assets

- application source and generated artifacts;
- server secrets and environment configuration;
- user request and response data;
- filesystem and network capabilities;
- native platform credentials and secure storage;
- handler runtime integrity;
- build and release identities;
- signing keys, provenance, and update metadata;
- developer worktrees and caches.

## Trust Boundaries

1. Untrusted project files to project discovery.
2. HTML, styles, components, and companion source to compiler frontends.
3. Compiler stages to staging and output directories.
4. Network clients to Tachyon's HTTP edge.
5. Server runtime to the selected Yon isolation backend.
6. Handler processes or a trusted Firecracker control program to environment,
   filesystem, network, microVM, and host resources.
7. View IR to web and native code generators.
8. Staged application bundle to the platform's local-origin web view.
9. Main-frame local content to the route-scoped native capability bridge.
10. Generated platform projects to external mobile and desktop toolchains.
11. Installer and updater to release archives and the local filesystem.
12. CI jobs to third-party actions, registries, caches, and release identities.
13. Untrusted development source and diagnostics to the hot-update browser
    client.

## Primary Threats and Controls

| Threat | Required controls |
| --- | --- |
| Path traversal or output overwrite | project-relative canonical paths, symlink checks, staging directories, atomic publication, explicit output root |
| Compiler resource exhaustion | byte, node, depth, recursion, expansion, diagnostic, and time budgets |
| Template injection | structural parsing, typed bindings, context-aware escaping, no implicit eval |
| Malicious custom tags | bounded client render plans; no eval; compiler-owned AST; restrictive CSP |
| Route-context confusion | deterministic contributors, duplicate-key error, stable manifest, no last-writer-wins |
| Shell or argument injection | direct process spawning, fixed argv boundaries, no shell interpolation |
| Handler denial of service | bounded frames, deadlines, cancellation, concurrency limits, process memory/CPU policy, forced termination |
| Handler protocol desynchronization | four-byte length framing, maximum frame, strict schema, request IDs, stderr separation |
| Isolation downgrade by application source | parent-environment-only selection; no project or request override; invalid and partial policy fails closed |
| False Firecracker assurance | control-program path must be absolute, a regular file, not a symlink, and not writable by group or others; bounded policy arguments; hardware-isolation claims require separate driver and host evidence |
| Environment secret disclosure | allowlisted environment, redacted diagnostics, secret-canary tests; relay programs and Gateway commands never receive `YON_APPLICATION_JSON` or `YON_RESOURCE_*`. This scrubbing stops accidental inheritance only. It is not a sandbox: a program that runs as the same operating-system user can still read its parent's environment (for example `/proc/<parent>/environ` on Linux) and the project's files |
| Cross-language call confusion (`@Gateway`) | build-declared keys only, per-invocation bearer capability on a loopback endpoint, bounded JSON (1 MiB, depth 32, 10,000 values, 32 arguments), 64 calls and 4 concurrent calls per invocation, separate process pool, one hop, fixed error kinds with no target messages, refusal under Firecracker |
| SSRF and unrestricted egress | deny-by-default network capability, normalized origin allowlists, DNS/redirect revalidation, response limits |
| Native bridge escalation | declared capabilities; bounded per-route registry; operation/member/payload checks; trusted main-frame local origin only |
| Native origin confusion | canonical bundle resource mapping; frame and navigation guards; no remote bridge; no arbitrary file URLs |
| Cache poisoning or path replacement | versioned cache keys include compiler, contract, target, config, and source digests; directory capabilities that do not follow links; cleanup that checks identity; verified atomic writes; private runtime copies |
| Malicious archive | bounded extraction, path and symlink validation, explicit manifest, digest verification |
| Supply-chain compromise | locked dependencies, advisory/license/source policy, pinned actions, minimal release permissions, SBOM, provenance, signatures |
| Compromised release job | protected environments, tag/version verification, native builders, attestations, post-upload verification |
| Diagnostic data leakage | allowlisted structured fields, bounded snippets, no secrets or full environment dumps |
| Development hot-update injection or stale execution | versioned typed messages, JSON serialization, text-only diagnostic rendering, same-origin generated modules, compiler-owned island boundaries, last-good output, and reload on ambiguity |

In this table, "fails closed" means that the system refuses the action when it
is not sure. SSRF means server-side request forgery. SBOM means software bill of
materials.

## Reconciled request and browser boundaries

This section describes controls that are implemented. These documentation
updates are not a new security assessment or sign-off. The user waived the
separate cybersecurity review for this reconciliation.

- **CHEX.** CHEX validates fixed, captured schemas against private request
  files. Request bodies and authorization headers never enter argv or public
  errors. Admission and execution share one deadline. Concurrency has a
  limit. An invalid schema or a tool that fails to start fails before
  publication and before the socket is ready.
- **Browser response storage.** Storing responses is opt-in, and only for
  same-origin GET or HEAD requests without credentials. These skip storage:
  requests with Authorization, requests with cookies, private or no-store
  responses, bodies that are too large, and unsupported Vary responses.
  `Range`/`If-Range` requests skip both cache lookup and storage, including the
  offline fallback. The cache excludes `206` responses and responses with
  `Content-Range`, both when writing and when reading existing cache records.
  Browser storage is not a secret store. `@SessionState` and `@LocalState`
  fields, built-in Cache API caches and IndexedDB Stores must not contain
  secrets.
- **Pub/sub.** Published and retained messages are bounded. They are detached
  on abort, unmount and HMR (hot module replacement). Sharing a topic name does
  not give them native authority.
- **Native services.** Native services are trusted application code with host
  privileges. They are not sandboxed plugins. Checks on origin, route, service
  and owner limit who can call them. These checks cannot make malicious
  compiled application code safe.
- **Android.** Android pins AndroidX WebKit `1.14.0`. It uses that library's
  frame-aware asynchronous message listener and its reply proxy that is bound
  to the document. There is no fallback to the legacy JavaScript interface. A
  WebView runtime without support reports that the bridge is unavailable.
  Limits: at most 128 calls per document, each with a ten-second deadline. One
  worker accepts at most 128 queued requests. Navigation drops pending replies
  and stale queued work. Destruction shuts down the worker.
- **Apple.** Apple hosts and Swift services share Foundation JSON decoding and
  canonical request serialization. They do not use the old handwritten value
  scanner. Before decoding, a byte-level guard enforces a 64 KiB limit and a
  64-level depth limit, and rejects duplicate root keys. Dispatch uses the same
  canonical payload that the host validated.
- **Native field snapshots.** Each field has its own revision number, so a late
  reply cannot overwrite a newer edit. The native-call timeout limits how long
  the browser waits. It does not stop or roll back native service code that is
  already running. Services must stay responsive. Recovering from hung native
  work may require restarting the application.
- **Old evidence.** ADRs 0018 and 0019 replace the historical native and WASM
  (WebAssembly) evidence later in this document. That evidence must be
  qualified again for this release.

## Invariants enforced across both architectures

- Invalid or ambiguous security configuration fails closed.
- Remote content receives no native bridge.
- Compiler and handler inputs are untrusted regardless of repository origin.
- A successful build never writes outside its declared output and cache roots.
- A successful handler response belongs to exactly one request ID.
- Cancellation and parent termination eventually reap every owned child.
- Capability absence means denial.
- No support claim is based on cross-compilation alone.

## Phase 0 Evidence

- `unsafe_code = "forbid"` at workspace scope.
- Canonical contracts include rejected adversarial examples.
- A policy checks dependency licenses, sources and advisories.
- GitHub Actions are pinned to fixed commits.
- Release permissions are the smallest possible and are set per job.
- `docs/SUPPORT_TIERS.md` records the public limitations.

## Phase 1 Evidence

- **One project capability.** Discovery opens one project capability (a handle
  to the project directory). It rejects symlinks at every component it
  traverses. It builds routes and layer diagnostics from bytes read through
  that kept handle. So if something replaces the root during the pass, it
  cannot redirect views, routes, components, shared assets, build or native
  configuration, or any of the five layer roots.
- **One snapshot.** Web and native compilers use the same owned snapshot.
  Native compilation does not run discovery a second time.
- **Development server startup.** The development server also uses one Project
  for its initial build, its routes, the selected root middleware, the
  captured worker schedules and the worker HandlerSources. A root swap cannot
  mix those startup inputs.
- **Task ownership.** The server owns the watcher and the set of worker tasks.
  It cancels them with HTTP shutdown, waits a bounded time for them to finish,
  and aborts anything left when it is dropped. No worker or rebuild task runs
  detached. Handler-stream bridges, hot-update SSE and topic SSE use the same
  cancellation token.
- **Shutdown order.** Cancellation happens before Axum's graceful wait for
  connections. One global deadline limits both HTTP and task settlement. The
  last part of that deadline is kept for aborting and draining both task
  registries. So an infinite response cannot keep a handler child alive or
  block watcher and worker cleanup.
- **Registry size.** The server regularly removes records of finished
  producers before shutdown. So registry size depends on live work, not on the
  total number of past requests.
- **Worker schedules.** A change to worker schedules takes effect when the
  server restarts, not during a web-only hot rebuild.
- **Infinite handler streams.** Shutdown signals an infinite handler stream and
  then drops it at once through its supervised process-group guard. Shutdown
  never polls producer work that ignores cancellation again. Draining the task
  registry yields at a bounded rate, so a backlog of tasks that are all ready
  cannot delay the hard deadline.
- **HTML input.** HTML input is limited to 1 MiB, must be UTF-8 and must not
  contain NUL. Scripts, inline event handlers, compiler control tags and
  unresolved bare components fail closed.
- **Atomic builds.** Builds write to a staging directory next to the output.
  They replace the published output only after every route and the manifest
  succeed. A failure keeps the previous output.
- **Development server defaults.** The development server builds before it
  binds a socket. It uses loopback by default and requires an explicit opt-in
  for other addresses. It serves only generated files. It sends CSP, frame,
  MIME-sniffing, referrer and cache headers. A server bound to a loopback
  address answers only a request whose `Host` name is a localhost name
  (`localhost` or a name under `.localhost`), an IP address, the host of
  `tac.origin`, or a name in `server.hosts`; any other name gets `403` with
  `TY1366`, and a `Host` that cannot be read gets `400` (ADR 0094). This
  closes DNS rebinding: a page on an attacker's name that re-points the name
  to `127.0.0.1` sends its own name as `Host`, so it cannot read pages, call
  Yon routes, open the hot-update stream or upgrade a WebSocket. The check
  runs before routing in the development, production (`ty start`), preview
  and error-page servers, and it does not depend on the browser. A server
  opened to the network with `--allow-non-loopback` checks `Host` only when
  `server.hosts` is set, so a reverse proxy keeps working. Each watched
  rebuild runs a fresh discovery and passes that Project to the compiler.
  Shutdown cancels the owned watcher and waits for it to finish before
  returning.
- **Capability probes.** Before the build (or the `--no-bundle` output check)
  and before binding the socket, the development server runs synthetic
  capability probes. The probes have a deadline and an output limit. They
  cover the deduplicated runtime requirements of routes, root middleware and
  configured workers. Static-only projects probe nothing. Firecracker
  readiness fails closed for an unsupported discovered language or an invalid
  driver. It does not claim that a remote microVM pool is healthy. The C# probe
  builds source owned by the framework in a private temporary project with no
  package sources. This proves that the SDK can build, instead of trusting a
  list of runtimes.
- **Tests.** Tests with the real binary cover missing routes and a raw
  traversal request, in addition to successful GET and HEAD behavior.

## Phase 2 Evidence

- **Handler sources.** Handler sources must be regular files inside the
  project, not symlinks, in one of the ten owned Yon languages. Tachyon
  checks UTF-8, NUL and a 1 MiB limit. Entry points must declare the fixed
  plain `Handler` type before runtime selection. Application modules carry no
  interpreted framework markers.
- **Source snapshot.** Tachyon copies the validated bytes into an owned source
  snapshot with the same shape as the project. Process adapters and the
  JavaScript/Python Firecracker source contract use that fixed copy. The
  authored absolute path stays only as diagnostic metadata. Tachyon never
  opens it again as the handler program or working directory. Standalone
  handler discovery captures the whole bounded `server/**` tree without
  following links. So relative imports and relay programs resolve only from
  the owned snapshot.
- **Process start.** Tachyon starts runtime programs directly, with fixed
  argument boundaries and no shell. It selects embedded adapters from the
  language of the discovered source. It rejects arbitrary interpreter
  registration, shebang execution, executable-handler fallback and guessing
  from class names.
- **Missing runtime.** A missing runtime is `TY2112`, both during startup
  preflight and if the runtime disappears later, just before spawn (a race).
  Terminal and JSON diagnostics name only the logical runtime or the relevant
  configuration variable. Structured failure events add only the runtime
  family and the `not_found` category. They exclude raw override paths, OS
  errors, environment values, authored source, request bodies and child
  output.
- **Other languages.** Programs outside the owned language set run only
  through an explicit `relay(command, request)` boundary or a `@Gateway`
  command. Tachyon reads child stdout and stderr at the same time, under fixed
  limits. Public failures are redacted. The supervisor keeps the absolute
  deadline and the full process-group cleanup boundary. The supervisor writes
  the time left into the request just before it writes the frame, so queue and
  start-up time count. Each runner measures that budget from its own start and
  keeps it private, so a changed request or a second relay cannot extend it.
  A Gateway command gets only the baseline environment and the explicit
  allowlist. A relay program gets the same, plus `YON_HANDLER_PROTOCOL` and
  the `NO_PROXY` and `no_proxy` values of its handler. The baseline is `PATH`,
  `LANG`, `LC_ALL` and `TMPDIR` on Unix, and `PATH`, `PATHEXT`, `SYSTEMROOT`,
  `WINDIR`, `COMSPEC`, `TEMP` and `TMP` on Windows. `YON_APPLICATION_JSON`
  and every `YON_RESOURCE_*` name are removed even when allowlisted (names
  compared case-insensitively). This scrubbing prevents accidental
  inheritance. It does not contain a hostile program: an outside program runs
  as the same operating-system user, so it can read its parent's environment
  (for example `/proc/<parent pid>/environ` on Linux) and every file the
  project can read. Run a program you do not trust under separate isolation.
  On Unix, a PHP relay maps every inherited descriptor above 2 to
  `/dev/null`, so a relay program cannot write into the handler's response
  stream. PHP sends its own warnings to stderr, never to protocol stdout.
- **Resource endpoint clients.** Every runner connects directly to the
  loopback endpoint and ignores `HTTP_PROXY`, `HTTPS_PROXY` and `ALL_PROXY`,
  so the bearer token never reaches a proxy. As a second guard, the supervisor
  adds `127.0.0.1` to `NO_PROXY` and `no_proxy` for a handler that has an
  endpoint. When the outgoing request guard is active (the servers started by
  `ty serve`, `ty start` and `ty preview`), the supervisor removes `NO_PROXY`
  and `no_proxy` instead, so a client cannot skip the guard to reach another
  port on `127.0.0.1` or any other host; the runners still reach their
  endpoint directly. Go's standard client never sends loopback addresses to a
  proxy, so the Go prelude points `http.DefaultTransport` at the guard
  (`YON_OUTGOING_PROXY`). A client that builds its own transport, and a raw
  socket, still connect directly (ADR 0094).
- **Gateways (ADR 0042).** A caller reaches another language only through its
  invocation's resource endpoint, with a key that the build declared and
  checked (`TY1312`). A class target runs as a generated unit under
  `server/gateways/` inside the owned snapshot (authored files there fail with
  `TY1310`). The unit is project code: it runs like a route, with its own
  storage capability and no Gateway table, so only one hop is possible. A
  command target runs without a shell, with the arguments on stdin and never
  on the command line, and files only from `server/helpers/`. The caller
  sees only `invalid`, `timeout`, `capacity` or `adapter`; the supervisor logs
  only `handler.gateway_failed` with that category. Closing an invocation
  cancels its calls and waits for their processes to be reaped.
- **Environment.** Tachyon clears the child environment. Then it rebuilds it
  from a small platform runtime baseline and explicitly named allowlist
  entries. Tests prove that a host value is absent by default and present only
  when allowed.
- **Protocol stdout.** Protocol stdout accepts either one UTF-8 JSON response
  frame, or a declared stream. A response frame has a four-byte big-endian
  length and is at most 16 MiB. A stream has at most 100,000 events, at most
  256 KiB per event and at most 64 MiB in total. Tachyon validates request IDs,
  kinds, versions, fields, bodies, headers, statuses and adapter error shapes.
- **stderr.** Tachyon reads stderr at the same time and keeps at most 64 KiB.
  Adapter console output, exceptions and syntax or load errors produce bounded
  public messages without tracebacks or environment values.
- **Deadline and cleanup.** One deadline covers semaphore admission, spawn,
  execution, response, pipe settlement and exit. Each call owns a POSIX process
  group or a Windows Job Object that kills its processes on close.
  Cancellation sends a typed frame, then waits a default grace period of
  100 ms. On success, timeout and cancellation, Tachyon ends and reaps the
  whole process tree within a bound.
- **One process per request.** Tests prove crash isolation, successful
  recovery through a new child, and forced termination of an infinite loop.
- **Adversarial tests.** Tests with real processes cover:
  - output smuggled after the response;
  - oversized output;
  - mismatched request IDs;
  - floods of stderr;
  - missing methods and classes;
  - syntax errors;
  - exceptions;
  - results that cannot be serialized;
  - concurrency queue expiry;
  - Unicode paths and payloads;
  - missing runtimes;
  - descendants that inherit the protocol pipes.

  For supervisor failures, HTTP dispatch returns only a request reference. A
  secret-canary test proves that process diagnostics do not cross the network
  boundary.
- **Stream failure.** A stream failure sends one final, sanitized SSE `error`
  event with a stable code and the request ID. This is the same whether the
  failure happens before the first value or after some values were sent.
  Subscriber disconnect and deadline expiry end and reap the whole process
  group.
- **Handler cache.** Compiled handler artifacts are published atomically into
  a capability-confined `.tachyon/handlers` cache. Tachyon counts the size of
  inactive entries recursively and prunes them to 256 entries and 512 MiB.
  Kotlin, Rust, Dart, Go and Groovy artifacts are also shared by digest name
  through `runtime/handlers-v1` in the user cache (ADR 0037 amendment,
  2026-10-03). Only regular, non-empty files are restored, without following
  symbolic links, so the user
  cache must stay private to the developer account, as it already must for
  the cached web builds.
  Before execution, Tachyon copies a cache hit into a private runtime
  workspace. Lock recovery checks process liveness, identity and the ownership
  token. A ten-minute lease limits the risk of PID reuse.

Process mode is supervision, not a security sandbox. Application handlers keep
the ambient filesystem and network access of the developer account that runs
them. The operating system does not enforce CPU or memory quotas in this mode.
Application dependency imports and process pooling are still out of scope. No
documentation may suggest that those controls exist.

## Environment-Selected Isolation Evidence

- `process` stays the default, for compatibility. `firecracker` requires a
  complete environment policy and an absolute control-program path that
  Tachyon runs directly. An invalid mode, path, pool, CPU, memory or egress
  setting fails as `TY2010`.
- The control program receives a length-prefixed Handler Protocol v1 request,
  bounded policy arguments that are not secret, a cleared environment, and the
  existing deadline and cancellation supervision.
- Only JavaScript and Python source reach the current driver contract.
  TypeScript and the prepared direct-language paths fail with `TY2010` before
  the driver starts. Tachyon does not treat private runtime-workspace paths as
  artifacts that can be transferred.
- Egress (outgoing network traffic) is deny-only. An allowlist cannot be
  requested until origin, DNS, redirect and response-limit enforcement has its
  own acceptance evidence.
- This boundary does not qualify a Firecracker deployment. The control program
  and the Linux host stay trusted components. They must separately prove their
  jailer, cgroup, namespace, seccomp, image, snapshot, credential and network
  policy.

## Phase 3 Evidence

- Template expressions use a bounded parser that accepts only JSON-like data.
  It allows no calls, assignment, construction, prototype access, raw HTML,
  shell or `eval`.
- Text and attributes are escaped for their output context. Component
  templates see only evaluated properties. Slotted children keep their parent
  scope.
- Yon handlers never run during builds and never add view context. Route
  Manifest v1 keeps only a deprecated empty context member for wire
  compatibility. It contains no values and no executable declaration.
- View IR, source maps, manifests, diagnostics and build state contain no
  handler response and no values that come from the server.
- Component props are explicit public client input. Modules use same-origin
  paths that the compiler generates. They mount against Tac DOM that the
  browser creates. They mark bounded failures. They never expose a native or
  server capability bridge.
- Incremental state can be thrown away and is not trusted. Reuse requires a
  supported state version, contained regular paths, and a matching SHA-256 for
  every route artifact. Routes backed by handlers are never reused.
- Tests enforce limits on source, expressions, template depth, component
  count, iteration, expanded nodes, rendered output, diagnostics, processes and
  protocol.
- Real Chromium evidence covers client-owned rendering, automatic activation,
  interaction replay, bounded failure marking and static `never` components.

## Phase 4 Evidence

- Native planning expands validated Tac component declarations without server
  rendering. It uses a fixed allowlist of adapters and assigns deterministic
  identifiers. It emits the smallest unsupported custom-element or island
  subtree as a WebSurface.
- `client/manifest.json` must be UTF-8, a regular file and not a symlink. It
  is limited to 1 MiB, and its `tac.application` member rejects unknown keys.
  Tachyon validates it before native publication. The manifest is published
  to every browser, so the application id, version and window settings in it
  are public by design; it must never hold a secret.
- These are bounded: controller names, scalar values, state entries, action
  verbs, numeric increments, node depth and count, surface count, fallback
  bytes, Swift source, tool output and lifecycle log fields. Invalid
  declarations fail before Swift compilation.
- Local WebSurfaces get a generated deny-by-default CSP, navigation limited to
  the bundle, a non-persistent store and no script message handler. Remote
  WebSurfaces require a strict HTTPS host, turn off content JavaScript, allow
  redirects only to that host, and always use `bridge: none`.
- Capability Manifest v1 declares `default_policy: deny`, no capabilities and
  `remote_content_bridge: false`. Artifact Manifest v1 records toolchains,
  contract versions, and every output path, size and SHA-256 digest.
- Native builds use contained staging directories and reject generated
  symlinks. They atomically replace only `dist/macos`. A failed build keeps the
  previous complete application.
- Real macOS evidence proves named Accessibility controls, keyboard input,
  action and disclosure state, lifecycle termination, isolated WebKit content,
  and parity with the required mobile-web roles and names.

## Phase 5 Evidence

- Every generated native host runs its `WebSurface` with a temporary,
  non-persistent data store. It turns on scripting only for local-bundle
  content. It cancels any navigation outside the bundle root or the declared
  HTTPS host.
- No generated host exposes a script message handler, a JavaScript interface
  or any other bridge to web content. Unit tests check, by name, that each
  platform's bridge API is absent.
- Linux local WebSurfaces use a private resource scheme instead of `file://`.
  Decoded paths must resolve to regular files, not symlinks, under the root of
  the surface that started the request or under the generated WebBundle root.
  On every platform, remote surfaces compare the HTTPS scheme, host and
  effective port.
- Capability Manifest v1 is emitted deny-by-default for every target, with
  `remote_content_bridge` false.
- Tachyon runs every platform toolchain directly, never through a shell.
- Tachyon rejects generated output that contains a symlink before publication.

## Phase 7 Evidence

- Four fuzz targets cover the trust boundaries named below. They ran
  7,197,692 executions in total under `AddressSanitizer`, with zero crashes,
  timeouts or out-of-memory conditions. Details are in `PHASE_7_EVIDENCE.md`.
  - `handler_frame` decodes frames written by an untrusted child process:
    4,482,194 executions.
  - `html_frontend` and `template_frontend` parse application source.
  - `native_planner` lowers resolved HTML and checks that all five native
    targets agree on what is valid. So the choice of platform cannot widen
    what Tachyon accepts.
- The library test suite passes under `AddressSanitizer`. `LeakSanitizer` and
  `ThreadSanitizer` run in CI on Linux.
- Recovery drills check that publication is atomic. An interrupted, failed or
  concurrent build never leaves partial or broken output. Tachyon detects a
  corrupted incremental cache instead of trusting it.
- The build generates a CycloneDX bill of materials. The release binary embeds
  its dependency list through `cargo auditable`.
- The `fuzzing` feature exposes crate-private parsers to fuzz targets only. It
  has no compatibility guarantee. A released artifact must never enable it.

## Semantic Hot-Update Evidence

- Hot Update Protocol v1 allows only `css`, `island`, `reload` and
  `diagnostics`. It cannot carry source text or an instruction to evaluate
  code.
- `--no-watch` turns off the development endpoint. The endpoint is
  same-origin, uses a bounded broadcast queue, and turns off proxy buffering.
  When events are missed or late, it sends a full reload instead.
- The browser writes diagnostics with `textContent`. It keeps the last good
  DOM when the compiler fails. It reloads when the number of island boundaries
  or a stable identity changes.
- Island replacement imports only same-origin module URLs that the compiler
  generated. Before its boundary is replaced, the old instance receives an
  abort signal and a disposal callback. State transfer uses `structuredClone`
  on either the explicit `hotState()` result or, by default, the enumerable
  fields that are not functions. If the default clone fails, the transferred
  state is an empty object. Tachyon claims no JSON or depth limit here.
- State transfer for browser-owned DOM saves at most the first 2,048 elements
  that have an `id`. It keeps input and textarea values, the checked state of
  inputs, `<details>` disclosure, nonzero scroll offsets, and the focused
  element by id. It does not keep text selection or the selected `<option>`.
  It cannot transfer markup, attributes, listeners, arbitrary DOM properties
  or contenteditable HTML.
- A real Chromium gate tests CSS retention, island replacement, disposal,
  retention of component and native DOM state, failed-build diagnostics,
  recovery and structural reload, with no browser console errors.

## Remaining Security Gates

- Automated security gates stay required for every release. An independent
  human assessment is recommended for major changes to trust boundaries, but
  it is not required before publication. Any accepted remaining risk must be
  recorded in the release review package.
- Enforce handler filesystem, network, CPU and memory capabilities before
  production routing.
- Run live adversarial tests of `WebSurface` against remote origins, DNS and
  redirects.
- Run secret-canary logging tests.
- Complete the platform sandbox and production signing reviews.
- Run scheduled long fuzz campaigns that carry the corpus from one run to the
  next.
