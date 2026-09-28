# Phase 1 Web Vertical Slice

## Outcome

A new developer can use one compiled Rust `ty` executable to create, build,
and serve a static Tachyon application. They do not need Bun, Node.js, a
handler runtime, or application dependencies.

## Public Workflow

```text
ty init hello --name "Hello"
ty build hello
ty dev hello
```

`ty bundle` is an alias for `ty build`. `ty serve` is an alias for `ty dev`.
These aliases keep the current command names while the rewrite is in
development.

## Source Model

- `client/pages/tac.html` is the root client page.
- `client/pages/<static-segments>/tac.html` is a nested static client page.
- A route segment contains only lowercase ASCII letters, digits, and hyphens.
  Two hyphens cannot be next to each other. A hyphen cannot be the first or
  last character.
- The compiler rejects symlinks below the client and server source roots.
- Every source file is UTF-8, is at most 1 MiB, and contains no NUL byte.

Phase 1 splits HTML into tokens with a tokenizer that is compatible with the
WHATWG standard. The compiler places an HTML fragment into a fixed, accessible
document shell. The output is the same every time. The compiler keeps a
complete document as it is, after it normalizes the line endings.

The compiler rejects tokenizer errors. It also rejects any feature whose
behavior belongs to a later phase:

- `if`, `else`, `for`, and `loop`;
- Tac components that do not have a standard custom-element name;
- companion `tac.*` source;
- inline scripts and event-handler attributes;
- dynamic route segments.

## Build Contract

`ty build`:

1. converts the project root to its canonical path;
2. finds the sources without running application code;
3. builds a sorted route graph that cannot change afterward;
4. tokenizes and validates every HTML source;
5. writes each static route as `<route>/index.html`;
6. writes `route-manifest.json` in the Route Manifest v1 format;
7. publishes the full output directory only after every route succeeds.

The same source and options always produce output that is identical byte for
byte. A failed build does not change the output that was published before.

## Development Server

`ty dev` builds the project before it binds to an address. It serves only
generated files. The server:

- binds to `127.0.0.1` by default;
- supports GET and HEAD;
- returns a 404 response of limited size;
- sends browser headers that improve security.

To bind to an address that is not loopback, you must pass
`--allow-non-loopback`.

## Diagnostics

A failure has:

- a stable `TY####` code;
- source locations relative to the project;
- help that tells the developer what to do;
- a non-zero exit status.

`--diagnostic-format json` prints the Diagnostics v1 format. The default human
format is made for reading in a terminal.

## Acceptance Criteria

- A generated project builds twice, and both outputs are identical byte for
  byte.
- The compiled Rust binary serves the project's root page and route manifest.
- The behavior-level test suite runs the real `ty` process.
- These cases fail safely:
  - missing sources;
  - route collisions;
  - unsafe source shapes;
  - unsupported syntax;
  - unsafe bind configuration;
  - an initialization target that is not empty.
- The workspace passes formatting, check, Clippy, tests, rustdoc, coverage,
  dependency policy, and the legacy compatibility suite.
- Linux, macOS, and Windows CI (continuous integration) build and test the same
  generated project.

## Out of Scope

Later phases add these features:

- handler execution;
- context injection;
- control tags;
- components;
- bindings;
- islands;
- incremental rebuilds;
- source maps;
- native rendering;
- remote binding without an explicit opt-in;
- TLS;
- production HTTP serving.
