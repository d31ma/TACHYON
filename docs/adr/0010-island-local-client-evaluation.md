# 0010 — Island-local client evaluation

## Status

Superseded by [0015](0015-tac-is-exclusively-client-rendered.md).

## Context

A component template renders before any companion runs. A field that the
companion assigns is therefore never in render scope. Building the project's
own website showed the cost clearly. After we closed every parser gap, 19
diagnostics remained, and this one cause explained most of them. A panel that
gets its data from its companion cannot render at all.

ADR 0009 declined to reproduce the legacy client renderer. That decision still
stands for pages. A page is a real document: the server renders it, and
navigation uses view transitions. But an island is already the declared
boundary where client behaviour is allowed. If nothing can be evaluated there,
the boundary cannot do the one job it exists for.

We considered two options.

**Migrate the affected views to the server-rendered model.** This is correct
for an application that has a Yon route behind the view. It is wrong for the
frontend-only case. In that case no handler exists to compute the value. For
the website, it would also delete the polyglot gallery, and that gallery is
the site's content.

**Let an island evaluate against its companion.** The island is already a
declared boundary with a module, a lifecycle and a hydration policy. Adding
evaluation inside it changes no page, no route and no document.

## Decision

Inside an island, and only inside an island, the compiler defers to the client
an expression that the server cannot resolve. The build does not fail.

The compiler serialises the parsed expression to JSON and emits it as a
marker. The runtime first constructs the island's companion. It then evaluates
the marker against that instance and fills in the result.

The decision keeps three properties:

- **Nothing is evaluated as source.** The compiler has already parsed the
  expression into a bounded AST (abstract syntax tree). The client interprets
  that AST. The runtime has no `eval`, no `new Function` and no JavaScript
  parser. Event bindings already use the same approach.
- **The boundary is declared, not inferred.** A page, a route view and a
  non-hydrated component still resolve every expression on the server. They
  still fail the build when they cannot. Deferral requires `hydrate`.
- **Server rendering stays preferred.** The server resolves every expression
  that it *can* resolve. That content is in the document before any script
  runs. Deferral is a fallback, not a mode.

A deferred expression may contain a method call. The reason: the instance that
the call resolves against exists on the client. A deferred expression may
also use `await`. Client evaluation is async, so an awaited companion method
resolves. Everywhere else, the compiler still refuses both, for the same
reason as before: at render time there is nothing to call and nothing to
await.

## Consequences

- A deferred value is absent from the server-rendered HTML. It is not in the
  initial paint and not in view-source. A client with scripting disabled does
  not see it. An island already had this property for its behaviour. It now
  also has it for the deferred parts of its content.
- The runtime gains an expression interpreter. The interpreter accepts only
  the grammar the compiler accepts. It therefore cannot grow into a general
  evaluator.
- This decision does **not** cover control flow inside an island, such as
  `<loop>` and `<logic :if>` over companion data. To defer a subtree instead
  of a value, the build must ship the template. That is a larger change and
  needs a separate decision.

## Addendum: assignment and refresh

A binding inside an island resolves against that island's companion. It may
assign to a field of that companion. After a handler or an assignment runs,
the island re-evaluates its own deferred expressions and updates their text
nodes.

This is deliberately not a reactivity graph. An island's expressions form a
small, fixed list that is known at build time. Re-running all of them is
simpler to implement and cheaper to execute than tracking which expression
depends on which value. There is no dependency registration, no invalidation
and no diffing. The island's own template limits the cost, not the page.

Outside an island, assignment stays refused, because there is no instance to
write to. Only a binding outside every island requires the page's client
module. An island's bindings resolve on its companion.
