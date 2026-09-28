# 0017 — Mandatory Yon layers, explicit relay, and declared streaming

Status: Superseded by [ADR 0020](0020-decorator-free-authoring-conventions.md); the Yon layer folders it mandated are shared per realm and no longer sit beside a route, amended by [ADR 0031](0031-realms-mirror-each-other.md); streaming is declared with `@StreamRoute` and a finished stream sends `event: end` ([ADR 0058](0058-streams-are-a-route-and-a-request.md))

## Context

The old design had three problems.

First, arbitrary `.tachyonrc` interpreters and executable or shebang handlers
made every file a possible runtime boundary. They also left routes,
middleware, and workers outside the five-layer architecture that the project
layout already expressed.

Second, a class-name fallback made a missing architecture declaration look
valid. Relay code written in each language could also expose child stderr, or
could deadlock when a pipe was full.

Third, the server must know about streaming before it sends HTTP headers. If
it finds a generator only after invocation, it is too late.

## Decision

### Languages and layers

Yon runs exactly these languages: JavaScript, TypeScript, Python, Java, C#,
Kotlin, PHP, and Rust.

Each layer folder requires one annotation and the matching class-name suffix:

| Folder | Annotation |
| --- | --- |
| `server/routes` | `@Controller` |
| `server/services` | `@Service` |
| `server/repositories` | `@Repository` |
| `server/clients` | `@Client` |
| `server/delegates` | `@Component` |

A layer may call only a deeper layer. Repository, client, and delegate are
peers.

### Relay

Programs in other languages can still run, but only behind an explicit
`@Relay` method on a delegate. Relay commands never use a shell. Their
request, stdout, stderr, deadline, and cleanup are bounded. Tachyon reads
stdout and stderr at the same time. Process details and stderr details never
enter a client response. The outer handler process group is the final
boundary for reaping descendant processes.

### Removed features

Tachyon removes `.tachyonrc.interpreters`, shebang handlers,
executable-handler discovery, and the `Handler` class fallback.
`.tachyonrc.workers` remains, but the scheduled source must use a Yon
language. Middleware has the same language restriction.

### Streaming

A multi-response HTTP method declares `@Stream` and yields values.

- JavaScript, TypeScript, Python, PHP, Kotlin, and C# support this shape.
- Java and Rust remain single-response.

Each yielded value is a bounded Handler Protocol event, sent as SSE
(server-sent events). Four cases end a stream: the deadline, a reader
failure, a subscriber disconnect, and a normal exit. In each case Tachyon
settles or terminates the complete process group within a bound.

### Runtime overrides

The runtime overrides are `YON_JAVASCRIPT_RUNTIME` and `YON_PYTHON_RUNTIME`.
The removed `TACHYON_*` spellings are not aliases.

## Consequences

- `TY2015` identifies missing layer declarations. `TY1502` identifies removed
  interpreter registration.
- `ty migrate check` tells authors to move arbitrary handlers to
  `@Component` + `@Relay`. `ty doctor` no longer checks interpreter
  registrations.
- The server knows whether it must emit SSE headers before it invokes a route.
- Compiled relay shims remain protocol preludes that each language owns. The
  Tachyon supervisor keeps final authority over deadlines and process-group
  cleanup.
