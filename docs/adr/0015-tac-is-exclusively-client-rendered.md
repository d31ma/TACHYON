# ADR 0015: Tac Is Exclusively Client Rendered

- Status: Accepted
- Date: 2026-08-10
- Supersedes: ADR 0006, ADR 0009, ADR 0010, and the production-rendering assumptions of ADR 0013
- Amended: 2026-10-03 (control elements), 2026-11-10 (`aria-*` booleans)

## Context

Tac is the browser application framework. Yon is the server framework.

In earlier phases, the server rendered Tac templates to HTML. The browser then
activated selected islands (small interactive parts of the page). This split
one view between two owners:

- The build or the server evaluated text and structure.
- The browser could refresh only selected expressions.

So when a companion changed the value behind an `if`, `else`, `for`, or
`loop`, the frontend could not reliably change the DOM.

The split also made the architecture unclear. A Tac application should not
start to render on the server just because the compiler can evaluate one
value at build time.

## Decision

Tac has no server-side rendering mode.

For every `tac.html` route, the compiler emits:

1. a bounded, versioned JSON render plan that contains the parsed template and
   the expression ASTs (abstract syntax trees);
2. a small bootstrap document that contains no rendered Tac view subtree; and
3. the shared Tac browser renderer.

The browser renderer creates the initial DOM and performs every later
rerender. It evaluates:

- interpolations and dynamic attributes;
- `<logic>` conditionals and `<loop>` iteration;
- switches, slots, and components;
- event bindings.

The renderer interprets AST data that the compiler owns. It never uses `eval`,
`Function`, or authored expression source.

The compiler may validate and serialize literal initial state. The compiler
must not:

- evaluate a Tac expression;
- choose a Tac conditional branch;
- expand a Tac loop;
- emit a rendered Tac component subtree into the response document.

### The `hydrate=` attribute

The compiler still accepts the existing `hydrate=` component attribute for
now. It is a compatibility spelling for a browser **mount schedule**. It does
not identify server HTML, and it does not allow hydration. New runtime APIs
use `mount` and `tachyon-component`.

The runtime may call a legacy companion `hydrate(root, signal)` method after
the browser has created the subtree. This call is only a compatibility
lifecycle hook.

### Yon and native targets

Yon is outside the rendering pipeline. It dispatches REST endpoints and passes
their validated HTTP responses through unchanged; see ADR 0016.

Native targets run the same client render plan in platform web views; see
[ADR 0018](0018-native-hosts-run-the-platform-javascript-engine.md). They must
not get Tac structure by asking the web compiler to render it on the server
(SSR, server-side rendering). Target-native page companions supply platform
behavior under ADR 0019.

## Consequences

- Changes to Tac structural state work entirely in the frontend.
- Tac output has one owner. The whole class of hydration-mismatch bugs is
  gone.
- A browser without JavaScript receives an explicit `noscript` message. It does
  not receive a partly interactive application.
- A Yon handler may return HTML explicitly. Tachyon does not interpret that
  body as a template.
- Existing SSR and island snapshots, and hot-update assumptions, must move to
  the client render-plan boundary.
- Content Security Policy (CSP) stays manageable for three reasons: the render
  plan is data, the runtime is an external file, and authored expressions
  never run as source code.

## Rejected Alternatives

- **SSR only for static-looking Tac nodes**: Ownership would depend on what the
  compiler infers. Hydration behavior would come back.
- **SSR conditionals but client loops**: One template would have two
  incompatible structural models.
- **An opt-in SSR flag for Tac**: This breaks the framework boundary. Deployment
  behavior would then depend on the flag.
- **Send expression source and call `eval`**: This weakens CSP. Templates would
  become an unbounded way to execute code.

## Acceptance Gate

- A Tac response contains no rendered authored view subtree and no island
  wrapper.
- Browser tests prove initial and reactive rendering of interpolations,
  conditionals, loops, components, and events.
- A Yon route publishes the handler's validated HTTP response. It does no
  template rendering and no value injection.
- Native planning does not consume server-rendered Tac output.
- Repository docs describe Tac as client-only and Yon as REST-only.
- These gates pass: canonical format, check, clippy, test, documentation,
  dependency policy, and browser.

## Amendment (2026-10-03): `logic` and `loop` are the only control elements

By user decision, a Tac view writes a conditional only with `<logic :if>`,
`<logic :else-if>` and `<logic else>`, and a loop only with `<loop :for>`.
The `<if :when>`, `<else>` and `<for :each>` elements are removed. This
overrides the compatibility requirement in `docs/RECONCILIATION.md`, which
kept them as legacy spellings. `<switch>` and `<case>` stay unchanged.

- The template compiler refuses `<if>`, `<else>` and `<for>` with TY1367. The
  message names the canonical form.
- The static HTML frontend no longer treats them as controls. They are unknown
  elements there (TY1106).
- `ty migrate check` reports them as `view.removed_controls`, and
  `ty migrate fix --write` rewrites them in every `client/**/tac.html`.
  `<for :each="item in items">` becomes `<loop :for="item of items">`; a
  counted loop keeps its declaration.
- Text inside a Yon response body is data, not a Tac view, so a response that
  contains `<if>` text is unchanged.

## Amendment (2026-11-10): a bound boolean on an `aria-*` attribute writes `true` or `false`

A dynamic attribute follows the HTML rule for true-or-false attributes: `true`
writes the attribute with an empty value, and `false`, `null` and a missing
value leave it out. That rule is wrong for WAI-ARIA. `aria-expanded`,
`aria-pressed`, `aria-current` and the other `aria-*` states take the text
`true` or `false`, and a screen reader reads an empty value as false. So
`:aria-expanded="menu.open"` told a screen reader that an open menu was
closed, and a closed menu had no state at all.

- On a dynamic attribute whose name starts with `aria-`, `true` writes the
  text `true` and `false` writes the text `false`. This covers every `aria-*`
  attribute, such as `aria-expanded`, `aria-checked`, `aria-pressed`,
  `aria-selected`, `aria-invalid`, `aria-current`, `aria-hidden`,
  `aria-disabled`, `aria-busy` and `aria-modal`.
- A string or a number keeps its text (`aria-current="page"`,
  `aria-level="2"`). `null` and a missing value still leave the attribute out.
- Every other attribute keeps the HTML rule, and `checked`, `disabled` and
  `selected` still set their DOM property too.
- A static attribute is written as authored; the build's accessibility check
  (ADR 0074) still asks a static `aria-*` true-or-false value to be `true` or
  `false`.
- The one browser renderer (`BoundAttribute` in `tac-client.js`) holds the
  rule, so it applies on the web, in every native host's web view (macOS,
  Windows, Linux, iOS, Android) and in a component exported as a custom
  element (ADR 0086). A service in any language returns its own boolean type,
  such as a Go `bool` field, and needs no string helper.
- The contract is the same in every Tac service language. A field of the
  language's own true-or-false type reaches the template as a JavaScript
  boolean, and the rule above writes it:

  | Language | Field |
  | --- | --- |
  | JavaScript, TypeScript | `expanded = false`, `expanded: boolean` |
  | Rust | `expanded: bool` |
  | Go | `Expanded bool` |
  | C# | `public bool Expanded` |
  | F# | `member val Expanded = false` |
  | Kotlin | `var expanded: Boolean` |
  | Java | `boolean expanded` |
  | Swift | `var expanded: Bool` |
  | Dart | `bool expanded` |
  | Ruby | `@expanded = false` |
  | Lua | `expanded = false` (`---@type boolean`) |
  | Python | `expanded: bool = False` |
  | C++ | `bool expanded` |

  No language is left out: none needs its own code, because the page runtime
  writes every bound attribute. Tests compile the same `MenuService` in each
  language that builds for the web (`tac_web::aria_binding_tests`, and the
  Python page tests) and check that `aria-expanded` reads `false`, then
  `true` after `toggle()`; the JavaScript service runs in the browser test
  (`scripts/tac-client-test.mjs`). The docs section "ARIA states from
  true-or-false fields" shows the service in every language.
