# ADR 0136: Tac Bindings Give Objects and Arrays to Custom Elements as Properties

- Status: Accepted
- Date: 2026-11-14
- Amended: 2026-11-14 (TAC-B1F: a refused property skips one binding, not
  the page; a failed render logs its cause)
- Amends: [ADR 0015](0015-tac-is-exclusively-client-rendered.md) (how the
  client writes a bound value) and [ADR 0086](0086-a-component-can-be-exported-as-a-custom-element.md)
  (a Tac view now uses the property door of an element)

## Context

A Tac view binds a value with `:name="expression"`. The client renderer
writes the value as an attribute. An attribute is always text, so the
client turns an object or an array into an empty string. Only `checked`,
`disabled`, `selected` and `value` are also set as properties.

Component libraries take rich data through JavaScript properties. DuVay's
`w-chart` takes its points as an array. The jobq status page (PLAT-1) wrote
`<w-chart :data="svc.points">`, and the chart got nothing (ticket TAC-B1).
Tac's own exported elements (ADR 0086) also take objects and arrays only
through a property.

There were two ways to fix this:

1. An explicit property binding, as Lit does: `.data="svc.points"`. The
   author chooses the door. It needs a new attribute kind in the compiler,
   in the client plan, in every check that reads view expressions, and in
   the docs. An author who writes `:data` still gets an empty string.
2. Set a non-text value on a custom element as a property, as Vue, Preact
   and React 19 do. The author writes `:data` as for any other value.

## Decision

1. The client renderer sets a bound value as a JavaScript property when
   both are true:
   - the element is a custom element: an element in the HTML namespace
     whose name has a dash (`w-chart`, `my-panel`);
   - the value is an object or an array (`typeof value === 'object'` and
     not `null`).

   In every other case the value is written as an attribute, as before.
   So a text, a number or a boolean on a custom element stays an
   attribute, which the element can observe and CSS can select. A plain
   HTML element (`div`, `input`) and an SVG element never get a new
   property.
2. The property name is the attribute name in camel case: `:data` sets
   `data`, `:chart-data` sets `chartData`. HTML makes attribute names
   lower case, so the dash is how a view names a camel-case property. This
   is the inverse of the kebab-case attribute names of ADR 0086.
3. The element gets the same object. The client does not copy, freeze or
   serialize it. It never writes the attribute for that binding.
4. When the view renders again and keeps the element (the update rules of
   "What an update keeps"), the client writes the property only when the
   value changed: a different object whose plain data differs, compared
   the way loop items are compared. A new array with the same values is
   not written again, so an element that renders on every write does not
   loop. A change made inside the same array, without a new array, is not
   seen; a service gives a new array or object to update the element.
5. When a binding stops giving an object (it gives `null`, `undefined` or
   text), the client sets the property to `undefined` first and then writes
   or removes the attribute as before.
6. When the element is not defined yet (its library loads later), the
   client still sets the property. When the definition arrives, the client
   upgrades the element and gives the saved value to the element's own
   setter, so the value is not hidden behind an own property.
7. An element can refuse a property: it has a getter and no setter, or
   its setter throws. DuVay's `w-sparkline` has only `get values()`, so
   `<w-sparkline :values="svc.points">` threw a `TypeError` (ticket
   TAC-B1F, found by PLAT-1). Before this amendment the throw stopped the
   whole render: the page stayed blank, the document got
   `data-tachyon-render-error="render_failed"`, and the console showed
   nothing. Now:
   - The client catches the failure for that one binding, skips it, and
     renders the rest of the page. The same rule holds for every write of a
     bound property: the first render, an update, clearing a removed
     binding, and the write after a late definition.
   - It logs one `console.error` that names the binding and the element,
     for example `Tac could not give :values to <w-sparkline
     id="b1-probe">`, with the error that the element threw. It logs once
     per element name and property for the life of the page, so a page
     that renders often does not fill the console.
   - It does not fall back to the attribute as text. The element did not
     take the value through its property, and Tac cannot know which text
     form the attribute expects (a comma list, JSON, or none at all). An
     object would also lose its identity. The author decides: the element
     gets a setter, or the view binds text, for example from a service
     getter that returns a string.
8. Every render failure that sets `data-tachyon-render-error="render_failed"`
   on the document reports its cause once with `reportError`, so the
   console shows it and the window's `error` event fires. This holds when
   the caller of the render (an activation, an event handler, a service
   change, `__tachyonTac.render()`, or the follow-up render of ADR 0137)
   handles the error itself. Before, only
   the start-up render's error reached the console, as an uncaught
   rejection; the start-up render now leaves the report to the renderer, so
   the cause is never logged twice. A blank page with a silent console is a
   defect.
9. There is no new syntax and no compiler change. The compiler already
   sends the expression to the client. Every path that renders a view uses
   the same client code: the first render, an update that keeps elements,
   an update with the focus inside, a hot update, the shadow root of an
   exported Tac element, and the native apps, whose web views run the same
   client.

## Consequences

- `<w-chart :data="svc.points">` gives the chart the array. Libraries that
  read properties work without wrapper code.
- A custom element that used to get an empty attribute for an object now
  gets the object as a property and no attribute. An element that read the
  empty attribute changes behavior; such an element got no data before, so
  this is a fix, not a break.
- A function is still not passed. Passing callbacks to elements can be a
  later decision.
- An element that wants text from an object gets the object. A view that
  wants text writes it in the expression, for example with a getter that
  returns a string.
- A binding that an element refuses shows as one console error, and the
  rest of the page still renders. The element keeps its own value for that
  property.
- The client gains about 3 KB of source, before compaction. `tac-web.js`
  does not change.

## Evidence

- `template::bound_property_tests`: in Node, only objects and arrays on an
  HTML custom element become properties; the camel-case name; the same
  object reaches the element; an update skips equal values, writes changed
  ones and clears a removed binding; a value given before the element is
  defined reaches its setter after the definition.
- `scripts/element-properties-browser-gate.mjs`
  (`bun run test:element-properties-browser`): a compiled Tac page in
  Chromium, Firefox and WebKit. A probe element gets an array and an object
  as properties, text and numbers as attributes; a `div` keeps the old
  rule; an update keeps the element and writes only the changed property;
  a removed binding clears the property; an element defined after the
  first render gets the value through its setter.
- `template::bound_property_tests` (TAC-B1F): a getter-only property never
  throws out of a write, an update or a clear, and gives one console error
  that names the element and the binding; the same after a late definition.
- `scripts/element-properties-browser-gate.mjs` (TAC-B1F): a `probe-fixed`
  element with a getter-only `values` gives exactly one console error in
  Chromium, Firefox and WebKit, keeps its own value, gets no attribute, and
  every other check on the page still passes.
- `scripts/runtime-browser-test.mjs` (TAC-B1F): a render started by
  `__tachyonTac.render()` that hits the loop item cap sets `render_failed`
  and reports its cause exactly once, although the caller handles the
  error; a failed follow-up render (ADR 0137) reports its own cause too.
