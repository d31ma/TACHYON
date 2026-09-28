# ADR 0008: Event Binding Contract

- Status: Accepted
- Date: 2026-07-27

## Context

Phase 3 refused every executable event attribute with `TY1306`. The
diagnostic told authors to use an island companion instead. This kept
arbitrary code out of views, which is a real invariant. But it left two
defects.

1. An author who wrote the legacy `on:click="increment()"` could not compile
   at all.
2. An author who wrote the compiled marker `data-tac-on-click` compiled with
   no error. The document then contained a dead attribute: the button
   rendered, but a click did nothing. Nothing reported the problem.

A silent no-op is worse than a diagnostic, because the author gets no signal
to act on.

The legacy compiler treats `on:<event>` as the authored form. It treats
`data-tac-on-<event>` as its own output. It uses that encoding so that
Light-DOM web components do not read a raw `on*` attribute a second time and
bind the handler twice.

## Decision

Accept `on:<event>` as the authored binding. Keep `TY1306` for anything
executable.

The compiler parses a binding value and never evaluates it. The grammar is a
handler name, optionally followed by literal arguments. The arguments may only
be strings, numbers, booleans or null. Anything else is a diagnostic. This
includes member calls, arrow functions, string concatenation and multiple
statements.

The compiler emits `data-tac-on-<event>` with the parsed handler name and
literals. It encodes the colon of a namespaced event as `__`, exactly like the
legacy compiler. A delegated runtime looks up the handler in the named exports
of the route's client module. The `tac.js` and `tac.ts` companions already
emit that module.

A route that binds an event but has no client module fails with `TY1306`. So
the dead-marker case cannot happen.

Native `onclick=` attributes stay refused. The diagnostic tells authors to use
`on:<event>`.

## Consequences

Authors can write ordinary event handlers again. The invariant that a view
never carries executable code still holds. The document contains only a
handler *name*. Only a module the author wrote can supply behavior. Handlers
live in a real module, so they are ordinary functions that authors can test
and type-check.

There are costs:

- The runtime delegates events from the document root. So a handler attached
  to a node that is later removed no longer fires. This matches delegation
  semantics and the legacy implementation.
- A binding to a handler name that the client module does not export is a
  runtime error, reported to the console. It is not a compile-time error,
  because the compiler does not parse the module's exports.

## Rejected Alternatives

We rejected evaluating the binding as a JavaScript expression, the way a plain
framework would. It would:

- bring arbitrary execution back into views;
- contradict the bounded-expression design that the rest of the template
  language uses;
- give injection attacks more places to target, with no benefit that the
  named-export form does not already give.

We rejected keeping islands as the only path. An island is the right tool for
a stateful subtree. But it is too much to require one for a single button.
Islands also do not explain why the compiled marker did nothing without an
error.

We rejected, for now, parsing the client module to check its exports at
compile time. That needs a JavaScript parser in the compiler. ADR 0007 already
answered this dependency question: use an external toolchain instead.

## Acceptance Gate

- [x] `on:click="increment()"` compiles to `data-tac-on-click="increment"`.
- [x] Literal arguments survive to the handler, and the event object is passed.
- [x] Executable values are refused with `TY1306`.
- [x] A route binding events without a client module fails closed.
- [x] Handlers fire in a real browser, proven by a CI gate.
