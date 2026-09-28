# ADR 0036: Views Read Shared Data and the Page Directly

- Status: Accepted
- Date: 2026-09-23
- Amends: [ADR 0030](0030-components-use-services.md) (a view's roots are
  no longer only the services it names)
- Preserves: bounded, compiled view expressions; services as the only home of
  behaviour; client-only Tac

## Context

A view could reach only the services that it named. So content that was only
data had to be a service. Examples are a list of questions, a footer's links
and the steps of a quickstart. The whole body of such a service was a literal,
in some language, returned from a field. The data had no behaviour to put in a
service. The service's language was simply whatever its author happened to
choose.

Services had the opposite problem with the page. A service that needed the
page's path, or needed to know whether the development server was serving the
page, read `location` or the document itself. That tied the service to a
browser. So the service could not be written in a language whose units run
outside a browser (Dart runs its units on the VM). It also could not reach the
page as WebAssembly (ADR 0035).

## Decision

- Every view has two built-in roots.
  - `data.<name>` is `client/shared/data/<name>.json`. The runtime loads it
    once per page as a JSON module, relative to the runtime. So it resolves
    the same way on the web and in a native web view.
  - `page.path` is the page's own path without a trailing slash.
    `page.development` is true while `ty preview` serves the page. The preview
    server marks the document it serves. A built document never carries the
    mark.
- When a view reads `data.<name>` and no file exists for it, the build fails
  with TY1311. A view can reach only files whose stems are identifiers.
- No service may be addressed as `data` or `page`. Discovery refuses
  `DataService` and `PageService`.
- A view passes either root to a service as a property, for example
  `<docs-pager :path="page.path" :docs="data.docs" />`. The service reads it
  through `RequiredArgsConstructor` like any other property.

## Consequences

- Content lives in `client/shared/data` as JSON. These are the same files
  that a JavaScript service already imported. A view reads them without a
  service.
- Some services needed the browser only to learn where they were. Such a
  service becomes pure logic over its properties. So it can be written in any
  browser language and unit-tested without a document.
- A page module or component that used `data` or `page` as its own name now
  reads the built-in root instead.
- Suppose a data file is removed but its route does not change. An
  incremental build that reuses the route does not revalidate it. A full build
  reports it.
