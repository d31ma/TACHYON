# Phase 3 Tac View Semantics

> ADR 0015 and ADR 0016 supersede this document. Tac is exclusively
> client-rendered. Yon is REST-only and never enters the view pipeline.
> ADR 0029 and ADR 0030 supersede the colocated `tac.js` activation module
> described below. Now a component directory holds `tac.html` and an optional
> `tac.css`. Behaviour is a `client/services/<Name>Service.<ext>` class that
> the view names.

## Outcome

The compiled Rust `ty build` command does four things:

- it lowers dynamic Tac HTML into deterministic View IR v1 (intermediate
  representation);
- it emits bounded client render plans;
- it publishes source maps;
- it reuses verified route artifacts that did not change.

It never invokes Yon handlers.

Phase 3 is a milestone for the build and for client render plans. It does not
add any of these:

- production HTTP handler dispatch;
- SPA (single-page application) navigation;
- native rendering;
- a compatibility implementation of every legacy expression.

## Template Language

Application developers still write HTML. A component template may use:

- escaped text interpolation: `{title}` or `{product.name}`;
- dynamic attributes and component properties: `:aria-label="title"`;
- conditional controls:
  `<logic :if="featured">`, `<logic :else-if="archived">`, and
  `<logic else>`;
- older conditional names, still accepted:
  `<if :when="featured">` followed by `<else>`;
- iteration controls:
  `<loop :for="product of products">`;
- older iteration names, still accepted:
  `<for :each="product in products">`;
- registered Tac components that use tags shaped like custom elements;
- component `hydrate` policies: `load`, `idle`, `visible`, `interaction`, and
  `never`;
- `<slot>` inside a component template.

New examples use `logic` and `loop`. The older `if`, `else` and `for` names
still work. The last branch is marked with a bare `else` attribute; `:else`
and `else="true"` are errors. Each `logic` element has exactly one branch
attribute. These controls go inside components. Pages place components and do
not use the controls directly.

```html
<logic :if="featured"><product-card /></logic>
<logic :else-if="archived"><archive-card /></logic>
<logic else><empty-card /></logic>
<loop :for="const product of products"><product-card :item="product" /></loop>
```

Text that is only whitespace, and comments, may appear between conditional
branches. An `else-if` or `else` produces a diagnostic if a conditional chain
does not come directly before it. Control elements are compiler syntax. They
never appear as ordinary elements in emitted HTML or View IR.

Limits:

| Limit | Maximum |
| --- | --- |
| Source size | 1 MiB (unchanged) |
| Expression length | 1,024 bytes |
| Expression nesting | 32 |
| Template/component nesting | 64 |
| Expanded nodes per route | 100,000 |
| Iterations per control | 10,000 |
| Generated route artifacts per route | 10 MiB |
| Diagnostics per build | 100 |

## Expression Contract

Tachyon parses expressions itself. It never passes them to `eval`, a shell, or
a language runtime. The Phase 3 expression grammar supports:

- JSON strings, finite numbers, booleans, and `null`;
- identifiers and dotted object paths;
- array indexes that use a non-negative integer literal;
- unary `!`;
- `===`, `!==`, `==`, `!=`, `<`, `<=`, `>`, and `>=`;
- `&&`, `||`, and parentheses.

Truthiness is defined over JSON values:

- `false`, `null`, numeric zero, and an empty string are false;
- arrays and objects are true.

Equality compares JSON values without coercion. `==` and `===` are aliases.
`!=` and `!==` are also aliases. Ordering accepts two numbers or two strings.

These produce a diagnostic that points to the source location:

- a missing key;
- an invalid index;
- an unsupported operator or call;
- a non-finite number;
- a type mismatch.

Every web view receives the immutable browser built-ins `platform`,
`environment`, `os`, and `target`. Each one is set to `"web"`. Yon does not
contribute view values.

Escaping and output rules:

- Interpolated values are HTML-escaped.
- Dynamic attribute values are attribute-escaped.
- `null` and `false` omit the attribute.
- `true` emits an empty attribute.
- Strings and numbers emit text.

The compiler rejects raw HTML interpolation, function calls, assignment,
constructors, prototype access, and executable event attributes.

## Yon Boundary

Yon is REST-only under ADR 0016. `ty build` discovers handler routes for
Route Manifest v1, but it does not invoke them. `yon.html`, route-context
composition, and the private `view.context` operation are unsupported.

A handler can explicitly return `Content-Type: text/html`. Then it supplies
opaque response bytes. Tachyon does not evaluate that body as a template.

## Tac Components

Components live below `client/components/**/tac.html`.

- Each directory segment uses lowercase ASCII letters and digits.
- The path segments, joined by `-`, form the tag.
- Phase 3 requires at least two segments, so that the name is a valid
  standard custom element name. For example:

```text
client/components/product/card/tac.html -> <product-card>
```

A hyphenated element that is not registered stays a web component. An unknown
element that is not HTML, not a control and not a component produces a
diagnostic.

Component properties:

- Static component attributes become string properties.
- `:property` attributes become evaluated JSON values.
- The reserved `hydrate` attribute is not a property.

The component template sees its properties and the render scope that the
browser owns. It gets Yon data through REST requests, never through an
implicit route context.

Invocation children replace `<slot>`. A component without a slot rejects
children that are not whitespace. Component expansion is recursive, checked
for cycles, and bounded.

An optional colocated `tac.js` is a browser activation module. Other `tac.*`
companions are still deferred. The module exports a default class:

```javascript
export default class ProductCard {
  constructor(props) {
    this.props = props
  }

  async mount(root, signal) {
  }
}
```

The `mount` method attaches behavior after the browser renderer creates the
component subtree. The runtime constructs the class once for each component
occurrence and calls `mount(root, AbortSignal)`. A legacy `hydrate` method is
accepted only as a lifecycle alias. There is no server DOM to hydrate.

## Component Mount Contract

A component that has a `tac.js` companion and no explicit policy activates with
`load`. A component without a companion stays static. Explicit policies work
as follows:

| Policy | Activation |
| --- | --- |
| `load` | when the external island runtime scans the document |
| `idle` | `requestIdleCallback`, with a bounded timer fallback |
| `visible` | intersection within a 100px margin, with immediate fallback |
| `interaction` | first pointer, keyboard, submit, input, or focus interaction |
| `never` | never loads a module or serializes properties |

Every scheduled component receives a deterministic occurrence ID. Properties
stay public, non-authoritative browser input. The browser renderer creates the
component DOM and then runs the scheduled companion lifecycle.

The runtime is an external module with no inline executable code. It checks
that generated module URLs are same-origin before it imports them. If
activation fails, the runtime:

- keeps the DOM that the browser rendered;
- reports a bounded error;
- allows a retry on a later interaction.

`hydrate` must be a literal supported policy. The build fails for a dynamic,
empty, or unsupported policy. It also fails for any activating policy without
`tac.js`.

## View IR and Source Maps

Each compiled page emits:

- its normal `index.html`;
- `.tachyon/view-ir/<route-key>.json`, conforming to View IR v1;
- `.tachyon/source-maps/<route-key>.map.json`, conforming to View Source Map
  v1.

View IR contains structural expressions, not evaluated values.

The source map maps byte ranges in generated artifacts back to byte ranges in
portable, project-relative source files. It may name a page or component
source. Its order is canonical. Generated wrapper markup has no invented source
mapping.

View IR and source maps never include Yon response values, runtime component
state, environment values, or application secrets.

## Incremental Build

Incremental reuse is on by default. The published `.tachyon/build-state.json`
records only canonical input digests and output digests. It never records Yon
response values.

A Tac route may reuse its verified bootstrap document, client render plan,
View IR, and source map when all of these are unchanged:

- its view;
- the complete component registry;
- the compiler version;
- the target;
- the relevant options.

Routes that have only a Yon handler produce no view artifact. They never run
during a build.

Reuse reads only regular files under the previous output. It verifies every
recorded SHA-256 digest and path. If a check fails, it recompiles. State that
is missing, malformed, of an unsupported version, symlinked, or has a digest
mismatch is a safe cache miss, not a build failure. Publication stays atomic. A
failed build keeps the complete previous output.

`ty build --no-incremental` turns off reuse. CLI output reports the number of
compiled routes and reused routes. Identical inputs still produce
byte-identical published artifacts.

## Diagnostic Recovery

These steps never run a Yon handler: discovery, parsing, structural
validation, expression validation, component resolution, and cycle checking.

The compiler collects independent source failures in canonical source/span
order, up to 100 diagnostics. It emits them through Diagnostics v1. A failed
build does not publish partial HTML, manifests, IR, source maps, runtime
assets, or incremental state.

## Stable Failure Families

- `TY1301`-`TY1399`: failures in template structure, controls, expressions,
  interpolation, attributes, or expansion;
- `TY1401`-`TY1499`: failures in component discovery, properties, slots,
  cycles, companions, or islands;
- `TY1501`-`TY1599`: reserved historical diagnostics; Yon has no view-context
  operation;
- `TY1601`-`TY1699`: failures in View IR, source maps, generated assets, or
  incremental state.

## Acceptance Criteria

- The compiled binary emits client plans for all of these: canonical and
  compatibility control tags, escaped interpolation, dynamic attributes, nested
  loops, condition chains, component properties, slots, web components, and all
  island policies.
- JavaScript and Python Yon handlers are discovered without execution.
  `yon.html` is rejected.
- These fail with stable diagnostics that point to the source: invalid handler
  response shapes, handler failures, component cycles, missing components,
  malformed controls, unsupported expressions, missing values, unsafe HTML,
  invalid islands, expansion limits, and output limits.
- Control tags never remain as browser custom elements. They lower to client
  render-plan instructions.
- Island IDs, HTML, View IR, source maps, manifests, assets, and build state are
  byte-identical for identical inputs.
- Incremental builds behave correctly:
  - an unchanged eligible Tac route is reused, and the reuse is verified;
  - a changed component invalidates the routes that use it;
  - Yon handler routes are not rendered;
  - corrupted state recompiles safely.
- Multiple independent template errors are reported together. Failed builds
  keep the last known-good output.
- Browser tests with the real binary prove client initial rendering, component
  mount scheduling, reactive structural updates, and interaction handling.
- These CI gates pass: formatting, check, Clippy, tests, rustdoc, coverage,
  supply-chain, legacy compatibility, macOS, Linux-container,
  Windows-buildability, and native.

## Out of Scope

Later compatibility or platform phases own these items:

- arbitrary JavaScript template expressions;
- raw HTML bindings;
- event bindings;
- general-purpose reactivity;
- two-way binding;
- Tac TypeScript/polyglot companions;
- component CSS scoping;
- SPA routing;
- production HTTP handler dispatch;
- handler pooling;
- native rendering;
- WebSurface planning;
- stable release packaging.
