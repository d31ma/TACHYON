# ADR 0015: Tac Is Exclusively Client Rendered

- Status: Accepted
- Date: 2026-08-10
- Supersedes: ADR 0006, ADR 0009, ADR 0010, and the production-rendering assumptions of ADR 0013

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
- `if`/`else` and `for`/`loop`;
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
