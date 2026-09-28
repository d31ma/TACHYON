# Phase 6 Evidence

This document records the validation of [`PHASE_6_SPEC.md`](PHASE_6_SPEC.md).
The resulting statement for each feature is in
[`PARITY_LEDGER.md`](PARITY_LEDGER.md).

Machine: macOS 26 (Darwin 25.5.0), Apple silicon, Rust 1.97.1, Bun 1.3.11,
Node 24, Playwright Chromium 1217.

## 1. Automated Gate

`cargo test --workspace --all-targets --all-features --locked`

| Suite | Coverage |
| --- | --- |
| `migrate` | clean projects; `unsupported` and `changed` classification with a required action; classification of view constructs from source; exclusion of generated and vendored folders; failure when the target is missing |
| `phase6_cli` | `TY1702` on unsupported constructs; `--allow-unsupported`; byte-identical JSON reports on repeated runs; the Migration Report v1 shape; classification of the archived migration fixture; a build and a clean migration check for every corpus project |

## 2. Differential Over the Shared Corpus

```bash
TAC_LEGACY_BIN=/path/to/26.30.04/ty \
TAC_BIN=target/debug/ty \
node scripts/compat/differential.mjs
```

Result: `3/3 corpus projects match across implementations`.

For `ty init`:

- 15 kept files are byte-identical;
- four files that relate to FYLO change on purpose;
- the three legacy `db/` files are removed on purpose.

| Project | Routes | Result |
| --- | --- | --- |
| `static-pages` | `/`, `/about` | Both build; both serve 200; semantic DOM identical on both routes |
| `semantic-html` | `/`, `/about` | Both build; both serve 200; semantic DOM identical for headings, `nav`, lists, links, `img`, `hr`, `details`/`summary`, `table`, and `footer` |
| `components-slots` | `/` | Both build; both serve 200; one declared divergence (see below) |

Each route is served over HTTP and rendered in the same Chromium build. We
measure the legacy output after its client runtime renders the DOM. We never
compare artifacts, because the two builds emit different files:

- The legacy build emits a single-page shell with a client router, a service
  worker, and one chunk per page.
- The Rust build emits bootstrap HTML for each route, a compiled render plan,
  and the Tac client runtime.

### 2.1 Declared Divergence

`corpus/components-slots/parity.json` declares exactly one divergence:

> `semantic dom /`: `<main>[1]: <product-card> vs <tachyon-component>`

Both implementations keep a runtime boundary element around each component.
This element has no semantic meaning. The legacy implementation uses the
custom-element name that the author wrote. The Rust renderer runs only on the
client and uses its own `tachyon-component` element as the boundary. Slotted
content, accessible names, and roles are identical. An undeclared divergence
fails the gate.

## 3. Behavioral Differences Found by This Work

We found these differences by running both implementations. We did not
assume them.

| Finding | Direction |
| --- | --- |
| The legacy HTML parser rejects standard void elements. `<img>`, `<hr>`, `<br>`, and `<input>` all fail with "No end tag." The legacy parser requires the self-closing form. The Rust parser accepts both forms. | The Rust implementation accepts everything the legacy one accepts, and more. Corpus fixtures use the self-closing form so that both implementations can be compared on the same input. |
| The legacy implementation has no `yon.html` convention. It treats every file under `server/routes/**` as a handler and rejects `yon.html`. | The Rust rewrite also rejects `yon.html`. ADR 0016 (architecture decision record 0016) defines Yon as REST endpoints only. |
| In the legacy output, Tac components use the custom-element boundary that the author wrote. In the Rust client, they use the `tachyon-component` runtime boundary that the framework owns. | Intentional; declared and recorded. |

## 4. `ty migrate check` Against the Archived Migration Fixture

```bash
ty migrate check crates/tachyon-cli/tests/fixtures/migration-fullstack
```

`crates/tachyon-cli/tests/fixtures/migration-fullstack` keeps the shape of an
archived full-stack project: handlers in several languages, services, and
workers. It is test data for the Rust migration checker. It is not framework
code that runs.

Result: **18 supported, 6 changed, 20 unsupported**, exit `TY1702`.

| Classification | Examples |
| --- | --- |
| `supported` | Tac views and bindings, JavaScript/Python handlers, middleware, workers, and controller companions |
| `changed` | Handlers in other languages that have no interpreter registration; client navigation between documents |
| `unsupported` | Legacy handler dependency imports, `OPTIONS.schema.json`, and built-in server telemetry |

Every finding that is not supported has a required action. The analysis
reads the project and never runs it.

## 5. Continuous Gate

The `compatibility` job in `.github/workflows/rust-ci.yml` runs on every pull
request. It:

- builds the CLI;
- installs Chromium;
- runs the differential over the corpus;
- runs `scripts/compat/standalone-rust.mjs`;
- classifies the migration fixture.

The standalone gate tests the compiled Rust binary. It verifies:

- the released set of commands;
- the scaffold;
- the cache lifecycle;
- the web build and server;
- the native bundle that contains only source;
- literal page-state activation and assignment in real Chromium;
- native controller state and capability behavior.

## 6. Released Standalone Workflow

```bash
TAC_BIN=target/debug/ty node scripts/compat/standalone-rust.mjs
```

Result: `PASS: released standalone workflow matches Rust ty (macos)`.

The gate clicks a page binding that assigns a value and waits for `Count: 1`.
The current regression contract requires every Tac route to emit:

- a render plan with a schema version;
- `/.tachyon/tac-client.js`;
- no authored subtree rendered by the server.

The in-tree JavaScript runtime and its test suite, which depended on its
implementation, were removed at cutover. The archived 26.30.04 executable
never changes. The neutral differential uses it. Public workflows that do not
need two implementations run directly against the Rust executable.

## 7. Open Gaps

| Gap | What closes it |
| --- | --- |
| Corpus coverage of conditionals, iteration, and mounted components | Add neutral fixtures whose inputs come from literal client state that both implementations accept. Yon route context is not available under ADR 0016. |
| Comparison of handler behavior between implementations | A handler differential that calls the same `yon.js` and `yon.py` through both supervisors and compares the responses. |
| Diagnostic parity | The legacy implementation raises runtime errors without types. Today we compare only whether both accept or both reject, not the messages. |

The `compatibility` job passed on pull request run
[30715759023](https://github.com/d31ma/Tachyon/actions/runs/30715759023).
That run included the released oracle, pinned by checksum, and the standalone
Rust workflow.

No ledger row moves to a new status without the evidence that the status
demands.
