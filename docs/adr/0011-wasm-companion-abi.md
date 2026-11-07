# 0011 — One wasm ABI for companions in any language

## Status

Superseded by [ADR 0019](0019-companions-compile-for-their-target.md). The text
below records the historical WebAssembly contract; it is not the current
companion build or runtime contract.

## Context

This ADR uses these abbreviations:

- **wasm** is WebAssembly.
- **ABI** (application binary interface) is the low-level contract between a
  compiled module and the code that calls it.

A browser cannot run a browser companion written in Rust, Kotlin, Swift, C#
or Dart as it is written. The legacy implementation solves this with a
hand-written subset transpiler for each language. That gives five partial
language implementations. Each one differs from the language it claims to
compile, and we would have to maintain all of them forever.

This implementation has refused that trade twice before:

- `tsc` emits TypeScript. We do not reimplement TypeScript, so the reference
  implementation defines the semantics.
- Polyglot handlers speak one direct protocol. A handler reads one JSON
  request from standard input and writes one JSON response to standard output.
  No language needs its own adapter. To add a language, you add a line in
  `.tachyonrc`.

Each of these languages already compiles to WebAssembly with its own
toolchain. So the question was never *whether* they can reach the browser. The
question was which interface Tachyon asks them to present once they are there.

## Decision

A wasm companion exports **three** things, and nothing else:

| Export | Signature | Purpose |
| --- | --- | --- |
| `memory` | standard linear memory | where requests and responses live |
| `tac_alloc` | `(i32) -> i32` | reserve `n` bytes, return the offset |
| `tac_invoke` | `(i32, i32) -> i64` | one JSON request in, one JSON response out |

`tac_invoke` receives the offset and the length of a UTF-8 JSON request. It
returns the offset and the length of a UTF-8 JSON response, packed as
`offset << 32 | len`.

The request is one of four operations. They match exactly what an island
already does with a JavaScript companion:

```json
{"op":"init","props":{}}
{"op":"get","name":"label"}
{"op":"set","name":"count","value":7}
{"op":"call","name":"loadingState","args":[]}
```

The response is `{"value": …}` or `{"error": "…"}`.

`init` answers with the module's members, for example
`{"fields":["count"],"methods":["doubled"]}`. The host therefore never has to
guess whether a name is a field or a method. Without this list, a read of an
absent field would return a callable instead of `undefined`. A plain object
does not behave like that. It would also hide a typo behind a call that fails
later.

This is the direct protocol, moved from a process boundary to a memory
boundary. The same idea removed eleven handler adapters: **one contract simple
enough that any language satisfies it directly** is better than an adapter for
each language. We write the contract once, not five times.

The host is a proxy. The island runtime wraps a wasm module in an object:

- a property read on the object becomes `get`;
- an assignment becomes `set`;
- a method call becomes `call`.

Everything above the object sees no difference between a wasm companion and a
JavaScript companion. Deferred expressions, event bindings, assignment and
refresh all work unchanged.

## Consequences

- **No bindgen, no component model, no glue crate.** Bindgen means a
  generator of binding code between a language and JavaScript. A language
  needs only two abilities: export two functions and read its own linear
  memory. Every wasm target provides at least that. Support therefore follows
  the language's own toolchain, not the coverage of a binding generator.
- **JSON at the boundary.** Values cross as text. Each access costs one
  serialise and one parse. An island's expression list is small. The runtime
  evaluates it at hydration and after events, not on every frame. User
  interaction limits the cost, not rendering.
- **A companion in wasm cannot touch the DOM (Document Object Model).** The
  companion computes, and the island renders. Under ADR 0010 this is already
  true of a JavaScript companion: the template owns the DOM and the companion
  owns the values.
- **Toolchains are a build requirement.** `ty doctor` reports them for each
  language before anything is compiled.
- **Module size is fine, once symbols are stripped.** The first measurement
  was 819 KB for a trivial fixture. That size would have made this approach
  unusable. Almost all of it was symbol names and metadata, not code.
  `-C strip=symbols` brings the same fixture to 21 KB. A module that avoids
  `core::fmt` goes down to 4 KB. Emission therefore always strips. The probe
  asserts a 64 KiB budget, so a test catches a size regression before a person
  has to notice it.
- **A `tac.rs` cannot serve both implementations.** The legacy bundler refuses
  a Rust companion that declares imports, because it feeds the file to a
  subset transpiler. A wasm companion is real Rust and needs imports. If you
  rewrite an existing companion to this ABI, the legacy build breaks. This is
  therefore a migration to do at cutover time, not before.
- `hydrate` is not part of the ABI. A wasm companion supplies values. A
  companion that needs to drive the DOM imperatively must be JavaScript or
  TypeScript.

## Rejected alternatives

**The WebAssembly Component Model with WIT (WebAssembly Interface Types).**
This is the standards-track answer, and it will be the right one eventually.
We rejected it for now because it requires a binding generator for each
language. Its coverage across these five languages is uneven. It would bring
back exactly the per-language dependency that this decision removes. A
component-model host can also satisfy the ABI above later, because the ABI is
a subset.

**A subset transpiler per language**, as the legacy implementation does.
Rejected above.

**Numeric-only exports with per-type accessors.** This avoids JSON. But it
needs a separate export for each field and each type. The module's exports
would grow with the companion instead of staying fixed at three.

## Amendment: a second module shape, for toolchains that emit only WasmGC

WasmGC is the WebAssembly extension for garbage-collected languages. When we
extended this ABI past Rust, we learned that the three exports above are not a
property of WebAssembly. They are a property of a toolchain that compiles to
**linear memory**. We measured this; we did not assume it:

| Toolchain | Exports `memory` | Imports | Bare instantiation |
| --- | --- | --- | --- |
| `rustc` → wasm32-unknown-unknown | yes | none | works |
| `dart compile wasm` 3.6.0 | **no**, 46 exports and none of them memory | 56, from `dart2wasm` and `wasm:js-string` | impossible |
| `kotlinc-js -Xwasm` 2.1.10 | yes | `js_code`, from the module it emits beside the wasm | impossible |
| .NET 9 `browser-wasm` publish | not a module at all — a runtime, the companion's assemblies, and a loader | many | impossible |
| swift.org `swiftc` → wasm32-unknown-wasip1 | yes | 14 `wasi_snapshot_preview1` | works, given an environment |

Dart is very clear about this. It rejects `@pragma('wasm:export')` in user
code as "for internal use only". A Dart companion therefore cannot declare a
wasm export at all. Kotlin exports its linear memory. But it still imports the
JavaScript that the compiler emits beside the module, so the module cannot
start on its own either. Neither gap will close. Both compilers target WasmGC
by design and ship a loader with every module.

A companion in one of those languages therefore presents a **glued module**:

> a JavaScript module exporting `tacInvoke(request) -> response`, where both are
> the same JSON text the bare ABI passes through linear memory.

Everything else stays the same: the same four operations, the same `init`
member list and the same proxy above it. The island runtime chooses by the
extension of the asset it receives:

- It instantiates a `.wasm` asset with no imports and drives it through
  `memory`.
- It imports a `.mjs` asset and drives it through `tacInvoke`.

That is one branch, with the same JSON on both sides of it.

The glue is the language toolchain's own output. This implementation does not
write it:

- `dart compile wasm` emits its loader;
- `kotlinc-js` emits its `.mjs`;
- the .NET publish emits a whole bundle.

The build ships that output beside a small entry module. The entry module
adapts whichever output the toolchain produced to `tacInvoke`. For each
language, we emit a wrapper of about twenty lines, not a language
implementation.

Swift teaches a second lesson. It compiles to linear memory and satisfies the
three exports exactly. But its compiler targets WASI (WebAssembly System
Interface). So the module imports fourteen functions that it never calls. It
also initialises through `_initialize` instead of at instantiation. A
companion author did not ask for either of these. The island runtime therefore
handles both: it supplies an environment that does nothing, and it makes the
`_initialize` call before first use. A module with no imports ignores all of
this. The alternative was to demand that every language that targets WASI
stop targeting it.

### An authoring prelude, not a subset

Neither language has reflection in a wasm build. A companion therefore
declares which members the island may reach. The build appends a prelude that
carries the ABI, and the author writes plain code:

```dart
int count = 6;
int doubled() => count * 2;

final tac = {
  'count': TacField(() => count, (value) => count = value as int),
  'doubled': TacMethod((arguments) => doubled()),
};
```

The real compiler compiles the prelude together with the author's source. The
prelude is a library, and the language stays complete. The legacy route does
the opposite: its compiler understands only a subset of each language.

### Consequences of the amendment

- **A glued companion cannot be verified by instantiating it in isolation.**
  The browser gate is therefore the check that matters.
  `scripts/wasm/companion-browser-test.mjs` drives Rust, Dart and Kotlin
  islands on one page. It asserts the same five behaviours for each.
- **Size follows the language's runtime, not the companion.** The same
  trivial fixture has one integer, one string and one method. Its size is:

  | Language | Size |
  | --- | --- |
  | Rust | 21 KB |
  | Dart | 98 KB |
  | Kotlin | 128 KB |
  | Swift | 5.5 MB |
  | C# | 3.5 MB across 21 files |

  Kotlin is 584 KB without `-Xir-dce` (dead code elimination). Swift would be
  53 MB with Foundation linked. Emission therefore passes the flags that avoid
  both. The 64 KiB budget in the ABI probe applies only to the bare Rust
  shape. Every other language brings its runtime. A companion author should
  know that before choosing a language.
- **A .NET companion boots instead of instantiating.** The bundle is a
  runtime. The island's first value therefore arrives after a runtime start,
  not after a module instantiation. The browser gate allows for this. A page
  that needs its content fast should not put a C# companion on the critical
  path.
- **Kotlin needs a standard library that the compiler does not ship.**
  `kotlinc-js` carries only `kotlin-stdlib-js.klib`. A wasm build needs
  `kotlin-stdlib-wasm-js` from Maven Central, and the project points
  `TAC_KOTLIN_WASM_STDLIB` at it. `ty doctor` reports when it is missing, so
  the build does not discover the problem later.
- **Values cross as JavaScript values where the boundary is already
  JavaScript.** The Kotlin prelude exports typed accessors, and the wrapper
  builds the JSON. Only the host ever serialises. Dart cannot do this, because
  it cannot export anything. Its prelude builds the JSON itself.
