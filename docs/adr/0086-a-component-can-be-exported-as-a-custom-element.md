# ADR 0086: A Component Can Be Exported as a Custom Element

- Status: Accepted (slice 1: components whose services are JavaScript or
  TypeScript; slice 2: a project without pages, TypeScript declarations and
  named slots; the items under "Later" are open)
- Date: 2026-09-29
- Relates to: [ADR 0029](0029-pages-compose-components.md) (a component is
  the unit of view, behaviour and style),
  [ADR 0030](0030-components-use-services.md) (behaviour lives in services),
  [ADR 0008](0008-event-binding-contract.md) (events),
  [ADR 0052](0052-the-manifest-is-the-tac-configuration.md) (the manifest is
  the Tac configuration)

## Context

A Tac component can only appear inside a Tac page today. Other teams build
their pages with plain HTML, React, Angular, Vue, or another Tac app. They
want to use one Tac component, such as a shopping basket, without moving
their page into Tachyon.

The browser already has a standard for this: a Web Component, which is a
custom element (a tag that JavaScript defines with
`customElements.define`). Every framework can place a custom element, give it
attributes and properties, listen to its DOM events, and put children inside
it.

This is step 1 toward micro-frontends (one page made from parts that
different teams build and deploy). Step 2 and step 3 are listed under
"Later".

## Decision

A component can be exported as a standard custom element.

### How a component opts in

The manifest lists the components to export in `tac.elements`:

```json
{ "tac": { "elements": ["shop-counter"] } }
```

Each entry is a component name: the component's tag, as a page writes it.
No annotation is needed, because a component is a view and a style, and it
has no class to annotate.

### The element name

The element name is the component's tag name. The folder
`client/components/shop/counter` gives `<shop-counter>`. The HTML standard
says that a custom element name must contain a hyphen. So a component in one
folder, such as `client/components/counter`, cannot be exported. The build
fails with `TY1358`, and the message says to move the component into a
folder with two parts. `TY1358` also reports an entry that names no
component.

### Props: attributes and properties

The element accepts these props:

- every name that the template reads, except service names, loop names,
  `data` and `page`;
- every field that a service of the component needs (`@NeedsAll`).

Each prop has two doors:

- An attribute. Its name is the prop name in kebab case (`persistId` becomes
  `persist-id`). An attribute value is always a string. Tachyon does not
  parse JSON in attributes.
- A JavaScript property with the prop name. It takes any value: a number,
  an object or an array. React 19, Vue and Angular set properties on custom
  elements, so rich values pass without any help.

The last write wins, whichever door it uses. When a prop changes, the view
shows the new value. A field that a service needs is read once, when the
service is created.

### Events out

A service sends an event with the application event publisher, as it does on
a Tac page (`this.applicationEventPublisher.publishEvent(model)`). When the
service belongs to an exported element, the element also dispatches a DOM
`CustomEvent`:

- The event name is the model name in kebab case: `CountChanged` becomes
  `count-changed`.
- `detail` is a copy of the model value.
- The event bubbles and is composed, so it leaves the shadow root and the
  host page sees it on the element.

Tac listeners still receive the event as before. Each element owns its own
events when its service is created for each element (`@NeedsAll` or
`@Lifetime('prototype')`). A singleton service is shared by every element on
the page, so its events come from the element that started it last.

### Slots and styles: shadow DOM

The element renders the component into an open shadow root.

- The template's `<slot>` becomes a native `<slot>`. The children that the
  host page puts inside the element stay in the host page's DOM. The browser
  shows them in the slot. So React or Vue keep full control of those
  children.
- A named slot, `<slot name="note">`, becomes a native named slot. A host
  child with `slot="note"` shows there (slice 2).
- The component styles (`.tachyon/components.css`) load once into one
  shared stylesheet. Every element adopts that stylesheet into its shadow
  root.
- Styles of the host page do not reach inside the element. Inherited
  properties (such as `color` and `font`) and CSS custom properties
  (`--brand-color`) do reach inside. Custom properties are the way to theme
  an element. The site-wide `tac.styles` do not apply inside an element.

We rejected light DOM (rendering into the element's own children). Host
styles would reach inside, but the host framework and Tac would both own the
same children: a React render would erase the Tac view. Slots would also need
Tachyon to move nodes by hand.

### Services and the runtime

Each element module imports the shared runtime `../tac-client.js`. The
browser loads one ES module once per URL, so a page gets one Tac runtime,
even with several exported elements, and even when it is also a Tac page.
Services load through the same loaders and from the same
`.tachyon/services/` folder as on a Tac page. The runtime also starts on a
page that has no Tac view plan.

An element whose component uses a service activates when it joins the page,
like `hydrate="load"`.

### Build output

There is no new build target. `ty build` and `ty preview` write one ES module
for each listed element, beside the files that a Tac page already uses:

```text
dist/.tachyon/elements/shop-counter.js
dist/.tachyon/tac-client.js
dist/.tachyon/services/...
dist/.tachyon/components.css
```

Beside each module, the build writes a TypeScript declaration file,
`dist/.tachyon/elements/shop-counter.d.ts` (slice 2). It adds the tag to
`HTMLElementTagNameMap`, declares a class `ShopCounterElement` with one
property per prop, and types `addEventListener` with an event map:
`count-changed` gives `CustomEvent<CountChanged>`. A prop has the type
`unknown`, because a template does not declare types. The event models
are the models that the component's JavaScript or TypeScript services pass
to `publishEvent`; the file copies each one as an interface, so it needs no
other file.

A host page loads the element with one script tag and uses the tag:

```html
<script type="module" src="/.tachyon/elements/shop-counter.js"></script>
<shop-counter start="5"><span>Apples</span></shop-counter>
```

### A project without pages

A project may only export elements. When `tac.elements` lists at least one
component, discovery accepts a project with no page and no controller, and
the build writes the element modules, the runtime and the services without
an `index.html` (slice 2). `TY1002` still stops a project that has no page,
no controller and no exported element.

### Named slots in Tac templates

Named slots are a Tac template feature, not only an export feature, so a
component behaves the same on a Tac page and in another app. `<slot
name="title">` takes a fixed, non-empty name (`TY1302` otherwise). On a Tac
page, a child element or component with a fixed `slot="title"` attribute
goes into that slot, and the other children go into `<slot>`. A child for a
slot that the component does not declare stops the build with `TY1404`, as
children without any `<slot>` already did. No new code is needed.

## Consequences

- A Tac component can be used by any web page, with standard browser
  features only.
- The runtime renders into several roots: the page body and the shadow root
  of each exported element. Focus stays on the focused field or button when
  an element renders again, as it does on a Tac page.
- A component that is exported must work alone: it gets its values only from
  its props and its services.
- A host page must serve the export from the same origin in slice 1.

## Acceptance (slice 1)

- `tac.elements` in the manifest, with `TY1358` for a name without a hyphen
  and for a name that is not a component.
- `ty build` writes `.tachyon/elements/<name>.js`.
- A plain HTML host page, tested in Chromium, Firefox and WebKit, passes a
  prop in, receives an event out, and shows slot content.
- A docs page and an example, checked by `verify-snippets`.

## Acceptance (slice 2)

- A project with `tac.elements` and no page or controller builds, and
  writes no `index.html`.
- `ty build` writes `.tachyon/elements/<name>.d.ts`; a TypeScript consumer
  that reads `event.detail` with the right type compiles, and one with the
  wrong type fails with `TS2322`.
- A named slot works in a plain HTML host page and on a Tac page, in
  Chromium, Firefox and WebKit.
- CI runs `scripts/web-components-browser-test.mjs`.

## Later

- Services in compiled languages (WebAssembly) inside exported elements,
  once each loader is tested there.
- Serving an export from another origin (CORS headers, and resolving service
  URLs against the runtime's own URL).
- Typed props in the declarations (from the types of the fields that a
  service needs) and event types for services in other languages.
- A `hydrate` attribute on the element (`idle`, `visible`, `interaction`).
- Fields that a service needs follow prop changes after the service starts.
- One stylesheet per element, with only the components it uses.
- Helpers for React and Vue types (JSX `IntrinsicElements`, Vue
  `GlobalComponents`).
- Form-associated elements (`ElementInternals`).
- Step 2 of micro-frontends: mount separately built Tac apps under route
  prefixes.
- Step 3 of micro-frontends: load remote components at runtime.
