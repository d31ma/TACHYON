# ADR 0035: Rust Services Reach the Web as WebAssembly

- Status: Accepted
- Date: 2026-09-23
- Amends: [ADR 0019](0019-companions-compile-for-their-target.md) (Rust is no
  longer desktop-only), [ADR 0030](0030-components-use-services.md) (the
  service target matrix)
- Preserves: the companion protocol (`init`, `get`, `set`, `call`, `dispose`),
  one Rust file compiled by `rustc` with no Cargo, the desktop hosts' native
  linkage, client-only Tac

## Context

A service in `client/services/<Name>Service.rs` reached macOS, Windows and
Linux, where the host links it. On the web, on iOS and on Android, the build
refused a service written only in Rust.

ADR 0019 had removed the old WebAssembly ABI (application binary interface)
because it did not reach the platform. But that reason is about native access,
not about the language:

- Rust has a first-class browser target.
- The companion protocol is already plain JSON through two C functions.

## Decision

### Building for the web

- A service with a Rust file and no browser script (js, ts or dart) reaches the
  web as WebAssembly.
- Every such service on the page compiles into one module with
  `rustc --target wasm32-unknown-unknown`. The module uses the same dispatch
  table and prelude that a desktop host links. It is built for size:
  `opt-level=s`, LTO (link-time optimization), `panic=abort`, and stripped.
- `ty doctor` checks for the `wasm32-unknown-unknown` standard library when a
  service needs it.

### Running in the page

- The page hosts the module. `.tachyon/services/tachyon-rust.js` embeds the
  module and speaks the companion protocol to it synchronously.
- Each `<Name>Service.js` default-exports a class with the Rust service's
  fields and methods. So the runtime renders, binds and disposes of it like any
  other browser service.
- Publications reach the page's application events through the service's
  publisher, after the Rust call returns.
- The module is embedded, not fetched. So it also loads from `file:` in a
  native web view.
- The page has one thread. So:
  - the behavior runtime takes its clock from the page (`performance.now`,
    `Date.now`);
  - a concurrency limit never waits;
  - a retry backoff spins (busy-waits).

### Native targets

- A native target that links a companion for the service uses that companion.
  It never imports the module.
- A native target that does not link one runs the module in its web view. This
  covers iOS, Android, and a target whose platform language is absent.
- The plan's `native` flag now means "linked on this build's target".

### Markers and members

- A Rust service declares its Tac markers the same way the other languages do.
  Each marker is on its own line above the struct:
  - `#[Service()]` gives its role, which lets a test unit compose it;
  - `#[Scope({"value":"prototype"})]`;
  - `#[RequiredArgsConstructor("field", …)]`.

  The build reads the markers and passes the struct to `rustc` without them.
- `init` carries the component's properties. The fields that
  `RequiredArgsConstructor` names receive them before `@PostConstruct` runs.
  This is the same on the web and in a desktop host.
- A method's parameters after its receiver cross as JSON when each one is
  `TacValue`, `String`, `&str`, `bool`, a float or an integer. A method that
  takes anything else stays inside Rust.
- `TacValue` defaults to `Null`, so a struct that holds one can derive
  `Default`.
- A Rust test unit combines the realm's Rust services with the prelude's
  `TacValue`, so a service that holds one can be tested.

## Consequences

- Every service language except Swift, Kotlin and C# now reaches every target.
- A Rust web service has no DOM or browser API. It is state and logic behind
  the protocol, and its view does the rest. `tac_local` values last as long as
  the page.
- Each call borrows the instance mutably, so `&mut self` methods are members
  too.
- A request is limited to 64 KiB, as it is in native hosts. So a property as
  large as a whole documentation corpus belongs in a Dart, TypeScript or
  JavaScript service instead.
- A Rust edit reloads the page. It does not hot-update one service the way a
  Dart edit does.

## Amendment (2026-09-30): models reach the view

- A field, a method argument or a method result may be a model from
  `client/models`, a `Vec` or `Option` of models, or a model that holds
  other models. It crosses through the `TacModel` trait that the build
  generates for each model, so a model derives `Clone`: the page reads a
  copy of the field. `TacValue` stays for values with no model.
- Other generic fields, such as `HashMap`, and path types still stay
  inside Rust.
