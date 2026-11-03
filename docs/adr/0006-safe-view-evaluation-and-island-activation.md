# ADR 0006: Safe View Evaluation and Island Activation

- Status: Superseded by ADR 0015 for Tac rendering; retained for Yon evaluation history
- Date: 2026-07-26

## Context

Phase 3 must:

- render handler data into HTML;
- lower structural controls;
- expand components;
- activate browser islands.

Tachyon could treat template expressions as JavaScript. That would have four
problems:

- builds would depend on a runtime;
- arbitrary code could run during rendering;
- native portability would be harder;
- there would be more places for injection attacks.

Island hydration also needs an explicit boundary in the browser. If startup
replaced the DOM that the server rendered, it would throw away useful HTML.
Deferred activation would also be visibly unstable.

## Decision

Parse expressions into a bounded Tachyon grammar. Evaluate them only over JSON
values. The grammar provides paths, indexes, literals, boolean operators and
comparisons. It has no calls, assignment, construction, prototype access or
raw HTML primitive. Escape text and attributes in the way that fits their
context.

Render every Tac component on the server. A component may have a `tac.js`
file. It exports a default class:

- its constructor receives public JSON properties;
- its `hydrate(root, signal)` method adopts the existing DOM.

A generated external runtime schedules activation for `load`, `idle`,
`visible` and `interaction`. `never` ships no module and no properties.

The compiler emits View IR before evaluation and source maps after rendering.
Neither artifact contains route-context values or serialized island
properties.

## Consequences

- Builds are deterministic across the JavaScript and Python handler languages.
- Expression errors point to a source location, and they cannot run
  application code.
- The future native backend reads the same View IR expression structure.
- The island DOM stays useful before and after an activation failure.
- The Phase 3 expression language is on purpose smaller than the legacy
  renderer, which is compatible with JavaScript. Later decisions must handle
  compatibility.
- Browser companions have a small class-based lifecycle API. It can be
  extended by adding things.

## Rejected Alternatives

- **JavaScript `eval` or generated functions**: compilation would no longer be
  deterministic, portable and resistant to injection.
- **Execute expressions independently in each handler language**: each
  language could interpret them differently.
- **Client-only components**: they hurt the first render, accessibility and
  failure recovery.
- **Replace SSR markup during hydration**: replacing the server-side rendered
  (SSR) markup makes the page visibly change. It also loses useful content
  when a module fails.
- **Inline executable hydration code**: it weakens the CSP (Content Security
  Policy) and makes auditing harder.

## Acceptance Gate

- expression parsing and evaluation have positive, negative, depth and size
  tests;
- text, attribute, JSON-script and URL contexts have adversarial escaping
  tests;
- every control is lowered, so none remains in the View IR element output;
- browser tests prove SSR adoption and failure preservation;
- source maps point generated ranges to page and component inputs;
- the threat model documents public island properties and handler execution.
