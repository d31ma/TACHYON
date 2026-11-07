# ADR 0009: Platform Navigation Over a Client Renderer

- Status: Superseded by ADR 0015
- Date: 2026-07-27

## Context

The legacy implementation ships a single-page application. It has a
1,060-line client renderer and a 605-line helper layer for the DOM (Document
Object Model, the browser's tree of page elements). Router, cache and
hydration code surround them. This code exists to give users two things they
can feel:

- navigation that is instant;
- navigation that is smooth.

The Rust implementation reaches those goals from a different starting point.
The build already prerenders every route to static HTML. A plain link
therefore works with no JavaScript at all. A client renderer would add speed
and polish, not new capability.

To reproduce the renderer, we would have to port the largest subsystem of the
legacy codebase. It is also the subsystem with the most defects. We would
also have to maintain a DOM reconciler forever. The result would only match
what the browser now does by itself.

`CONTEXT.md` states clearly that the rewrite is not a line-by-line port. Stable
behavior, source conventions and contracts carry forward. The internal
implementation structure is new, written from the beginning without legacy
code.

## Decision

Do not port the client renderer. Use the browser platform features that give
users the same two properties.

- Every generated page opts into cross-document view transitions with
  `@view-transition { navigation: auto; }`. A stylesheet delivers this rule.
- Every generated page carries speculation rules. The rules prefetch
  same-origin routes at moderate eagerness. As a result, the browser has
  usually fetched a route before the user clicks it.

A browser that does not support these features ignores them. Navigation then
works in the normal way.

The development server and the native hosts allow the rules block by the
SHA-256 hash of its text in their policy (CSP, content security policy).
Chromium's `'inline-speculation-rules'` keyword was used first, but WebKit
does not know it (ADR 0083 amendment). The rules payload is JSON, not
executable script. The policy therefore gains no general permission for
inline scripts.

## Consequences

Roughly forty bytes of CSS and one JSON block deliver the two user-visible
benefits. The alternative was 1,665 lines of client JavaScript that would need
testing, fuzzing and long-term maintenance. Pages keep working with
JavaScript disabled. No reconciler exists, so no reconciler can get out of
sync with the server's HTML. This removes a whole category of defect.

The costs are:

- State does not survive navigation the way it can in a single-page
  application. Each navigation really replaces the document. An application
  that needs state to last after a route change must keep it in an island, in
  storage or on the server.
- Prefetching uses more bandwidth to reduce waiting time.
- Browsers without support get correct but ordinary navigation.

This is a deliberate difference from the legacy implementation.
`PARITY_LEDGER.md` records it as a difference. It does not present it as
parity.

## Rejected Alternatives

- **Port the renderer.** Rejected for the reasons above. It has the highest
  cost and the highest risk in the codebase, and it only reaches what the
  platform already provides.
- **Write a smaller custom renderer.** Rejected because it combines the
  disadvantages of both options. It still owns a reconciler and can still get
  out of sync with the server's HTML. It also loses the one advantage of the
  legacy renderer: years of use and fixes in production.
- **Leave navigation with no improvement.** Rejected because users notice
  instant and smooth navigation. The platform now provides both at almost no
  cost.

## Acceptance Gate

- [x] Generated pages opt into cross-document view transitions.
- [x] Generated pages carry speculation rules that prefetch same-origin routes.
- [x] A same-origin route is prefetched before it is clicked, proven in a real
      browser.
- [x] No content-security-policy violation is reported.
- [x] Navigation still works with the features unsupported or JavaScript off.
