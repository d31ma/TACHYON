# ADR 0122: Python Services Reach the Browser APIs Through the Page

- Status: Accepted
- Date: 2026-11-11
- Amends: [ADR 0093](0093-python-is-a-tac-service-language.md) (Python Tac
  services had no browser API adapters) and
  [ADR 0087](0087-services-reach-the-window-and-the-network-through-injection.md)
  (browser APIs reach a service through injection)

## Context

Every Tac service language except Python could use Tachyon's browser APIs:
`Clock`, `SecureRandom`, `Connectivity`, `PageVisibility`, `Preferences`,
`Clipboard`, `Share`, `Notifications`, `Geolocation` and `WindowControls`. A
Python service that named one of them stopped the build with `TY1605`.

The page already has one host for these APIs, `web.host(text)`
(`template/tac-web.js`). Every language sends it the same JSON request,
`{"api": "clock", "call": "now", "args": []}`, and gets `{"value": …}` or
`{"error": "…"}` back at once. The page checks the `tac.browserApis` grants and
the user-action rule, and it sends late answers (such as `ClipboardTextRead`
or `PositionFound`) to the service that made the call.

Several features now being added (focus, files, push and others) need every
service language, Python included.

## Decision

1. **One host.** The Python engine passes the page's `webHost` to the Pyodide
   runtime as a `web` host (`python_host.js`, `_TacPythonRuntime.connect`). The
   runtime stores it as the builtin `TacWeb`. Python adds no second path to
   the browser, so the grants, the user-action rule and the late answers are
   the same as for every other language.
2. **Small clients.** Each API is a plain Python class with snake_case
   methods, such as `Clock.now_ms()`, `SecureRandom.bytes(count)` and
   `Clipboard.write_text(text)`. Each call goes through
   `_TacPythonWebHost.call(api, call, arguments)`. A refused call raises a
   `RuntimeError` with the page's message. `SecureRandom.bytes` gives `bytes`;
   a call that only starts work (such as `current_position()`) gives `None`.
3. **Injection.** A service declares a client as a `needs` field:
   `clock: Annotated[Clock, needs]`, or a typed field under `@needs_all`. The
   field is a constructor argument. The build describes it as
   `{"browserApi": "Clock", "name": "clock"}`, and the page passes a new client
   when it creates the service. Such a field is not a page property and the
   view cannot read it. A client-typed field without `needs` stops the build
   with `TY1605`, and the message shows the `needs` form.
4. **Event models.** A Python service may listen for every built-in browser
   event model (`ConnectivityChanged`, `VisibilityChanged`,
   `PreferencesChanged`, `ClipboardTextRead`, `ShareFinished`,
   `NotificationPermissionChanged`, `PositionFound`, `BrowserCallFailed` and
   `ApplicationStateChanged`). The build adds each model the program names as
   a dataclass.
5. **Test units.** A test unit has no page. When `TacWeb` is missing, the
   clients ask `BrowserFakes`: `BrowserFakes.answer("clock", "now", 1000)`.
   A call without an answer raises a `RuntimeError` that names
   `BrowserFakes.answer`. Because the client is a constructor argument, a unit
   passes it itself (`StampService(Clock())`) or passes its own fake object.
6. **Page-only clients stay refused.** `DocumentTitle`, `PreferenceStrings`
   and `FilePicker` have no Python client, so a Python service that names
   them still stops with `TY1605` (or `TY1407` for a `FilePicker` property).

## How to add a browser API to Python

The binding layer is `crates/tachyon-core/src/tac_python/python_browser_apis.rs`.

1. Write the client as one Python class in its own file under
   `tac_python/web_apis/`, named after the class (`focus.py` holds
   `class Focus:`). Give it a class constant `API` with the page's API name,
   and send each call with `_TacPythonWebHost.call(self.API, 'callName',
   (arguments,))`. Use snake_case method names. Convert the answer to the
   Python type the method promises.
2. Add one line to `PythonBrowserApis::CLIENTS`:
   `("Focus", include_str!("web_apis/focus.py"))`. The build then defines the
   class, registers it, gives it to `needs` fields, and stops refusing the
   name.
3. When the API has a new event model, add it to `WebEventModels`; Python
   gets it as a dataclass with no further work.
4. Add a case to `python_services_reach_every_browser_api_through_the_page`
   (`tac_python/browser_api_tests.rs`), which runs the built program under
   CPython with a recording `TacWeb` host, and a docs tab
   `client/services/<Name>Service.py` where the other languages have one.

A type added to `WebApiClients::TYPE_NAMES` without a Python client fails
the test `every_shared_browser_api_has_a_python_client`.

## Targets

Python services run only in the page, on Pyodide (ADR 0093). On the web, the
clients reach the browser APIs. A native app runs the same page in its web
view, so a Python service there reaches the same page host, and the host
answers the APIs it bridges (ADR 0089 for `WindowControls`). `WindowControls`
fails on the plain web with "needs the native app window", as in every other
language. No Python service is linked into a native host.

## Consequences

- Python joins the per-language tables of the browser API docs, with a tab
  in "Clock and secure random numbers", "Connectivity, visibility and
  preferences" and "Clipboard, share, notifications and geolocation".
- A Python call made after an `await` may run outside the call that the page
  is serving, so its late answer may go to every listener of the model, as
  for Dart.
- The browser API code for Python is about 150 lines of Python and one Rust
  list; new APIs add one file and one line.
