# ADR 0068: Safety Rules Are Build Errors

- Status: Accepted
- Date: 2026-09-27
- Preserves: [ADR 0048](0048-yon-routes-come-from-annotations.md) (routes
  come from annotations), [ADR 0055](0055-annotation-arguments-use-types.md)
  (annotation arguments are types), [ADR 0056](0056-every-annotation-checks-its-target.md)
  (every annotation checks its target) and
  [ADR 0062](0062-yon-uses-native-http-types.md) (Yon uses native HTTP types)

## Context

A framework that lets a mistake reach production has not done its job. Four
mistakes are common and dangerous in web apps:

1. A route that anyone can call, because nobody said who may call it.
2. A password, a token or a key written in the source code.
3. A query built by joining text, and code that runs a text as code (`eval`).
4. A server that sends requests to any host that a caller names (server-side
   request forgery, SSRF).

Each of them is easy to find in the source. So Tachyon refuses them when it
builds, the way it refuses a file with two classes. Each rule fails closed.

## Decision

### 1. Every route says who may call it

This is deny by default, like Spring Security.

- Three annotations say who may call a route. `@Public` means anyone
  (Spring's `@PermitAll`). `@SignedIn` means any signed-in user
  (`isAuthenticated()`). `@Allow(Role, ...)` means only a signed-in user
  with one of these roles (`@RolesAllowed`).
- They go on a controller class, on a route method, or on both. The rule on
  the method wins. A `@SocketRoute` class takes the rule on the class. A
  `@StreamRoute` method takes it like any other route.
- A route without a rule, directly or from its class, stops the build with
  `TY1322`. There is no switch that turns this off.
- `@Allow` takes role types, not strings, like the other annotation
  arguments (ADR 0055): `@Allow(Admin)`, `Admin.class` in Java,
  `Admin::class` in Kotlin and PHP, `typeof(Admin)` in C#. A role type is an
  empty class in `server/roles/<Role>.<ext>`, in any Yon language. A role
  with no type there stops the build with `TY1322`.
- The app has exactly one class marked `@IdentityProvider` (Spring's
  `AuthenticationProvider`), in `server/controllers/<Name>IdentityProvider.<ext>`,
  in any of the eleven Yon languages. Its one method `identify` (`Identify`
  in C# and Go) takes the native request of its language (ADR 0062), or one
  `@FromHeader`, `@FromCookie` or `@FromQuery` value, and returns the user and
  the names of their roles (`{"user": "ada", "roles": ["Admin"]}`), or no
  value when nobody is signed in. PHP reads `$_SERVER`.
- A `@SignedIn` or `@Allow` route without an identity provider, or a second
  identity provider, stops the build with `TY1323`.
- The server, not each language, enforces the rule, so it is the same for
  every language. Before a `@SignedIn` or `@Allow` route, the server calls
  the identity provider with the headers and the URL of the request. No
  identity answers 401. A user without one of the roles answers 403. A
  failing identity provider answers 500. In each case the route method never
  runs. A `@Public` route does not call the identity provider.
- The identity provider is compiled like a controller, with one internal
  route that is not in the route table, so no URL reaches it.
- A test unit calls a controller method directly, so it needs no identity.
  The identity provider is in scope in test units, so a unit can build it
  and fake its collaborators with `@Fake`.

### 2. No secrets in the source

- The build reads the code in `client/` and `server/`, the root middleware,
  the templates and the public configuration (`client/manifest.json`,
  `package.json`, `server/application*.json`). A value with the shape of an
  AWS, Google or Azure key, a GitHub, Stripe or Slack token, a Slack webhook,
  a private key or a JSON Web Token, or a long random text (16 characters or
  more, two kinds of characters, high entropy) given to a name such as
  `password`, `secret`, `token`, `apiKey`, `accessKey`, `privateKey`,
  `credential` or `signingKey`, stops the build with `TY1324`. The message
  names the file and the line, never the value, and points to `.env` (the
  private settings, docs/APPLICATION_CONFIGURATION.md).
- There is no switch. A test or a guide may keep an obvious example value:
  the file has the comment `tachyon: example secrets`, and each matched value
  says that it is an example (`example`, `sample`, `fake`, `dummy`,
  `placeholder`, `changeme`, `notreal`). A real-looking value stays refused
  in a marked file.

### 3. No text runs as code, and queries take parameters

- `TY1325` refuses calls that run a text as code: `eval` and `new Function`
  in JavaScript and TypeScript, `eval` and `exec` in Python, `eval`,
  `instance_eval`, `class_eval` and `module_eval` with a text in Ruby (the
  block forms stay), `eval` and `create_function` in PHP, `ScriptEngineManager`
  and `.eval(` in Java and Kotlin, `CSharpScript` in C#, `evaluateScript` in
  Swift and Kotlin and `evaluateJavascript` in Dart. Go and Rust have no such
  call. Comments and strings do not count.
- `TY1325` also refuses a data-access call (`query`, `execute`, `exec`,
  `prepare` and their spellings) on a `DataSource` or a receiver named like
  one (`dataSource`, `db`, `database`, `cursor`, `connection`, `statement`,
  `pool`) whose query text is not one text literal or an UPPER_CASE
  constant: joining with `+` or `.`, `${...}`, `#{...}`, `$name`, `f"..."`,
  `$"..."`, `.format(...)`, `%`, `fmt.Sprintf` and `format!` are all
  refused. The values go in the parameter list. Redis keys and values are
  always parameters, so Redis needs no check.

### 4. Outgoing requests go only to declared hosts

- Page code lists its hosts in `tac.outgoing.hosts` (`client/manifest.json`),
  Yon code in `outgoing.hosts` (`server/application.json`, the path
  `yon.outgoing.hosts`). A host is a name without scheme or port, or
  `*.example.com` for the subdomains of `example.com`.
- Build time: an absolute URL literal that is the first argument of a request
  call (`fetch`, `WebSocket`, `EventSource`, `urlopen`, `requests.get`,
  `URI(`, `URL(`, `URI.create`, `Uri.parse`, `Net::HTTP.get`,
  `file_get_contents`, `curl_init`, `GetAsync`, `http.Get`, `reqwest::get`
  and the other spellings), or any argument of `http.NewRequest` or
  `HttpRequestMessage`, must name a listed host, or the build stops with
  `TY1326`. A URL whose host is built while the code runs (`https://${host}`)
  is refused too, because nobody can check it. A URL that is only text, such
  as a link, is not a request and is not checked.
- Run time, pages: the server adds the listed hosts to the `connect-src`
  rule of the page's Content Security Policy, so the browser refuses every
  other host for `fetch`, `WebSocket`, `EventSource` and `sendBeacon`. Without
  a list, only the page's own origin is allowed, as before. The desktop and
  mobile hosts (ADR 0052) keep their own built-in policy for now; the build
  check still covers their page code.
- Run time, Yon: the server starts a local guard (an HTTP proxy on
  127.0.0.1) and gives every handler process `HTTP_PROXY`, `HTTPS_PROXY` and
  `ALL_PROXY` (and `NODE_USE_ENV_PROXY=1` for Node). The guard answers 403 to
  every host that is not listed and forwards the others. It refuses by
  default: with no list, every outgoing request is refused. HTTP clients
  that read the standard proxy settings are covered (JavaScript and
  TypeScript, Python, Ruby, PHP with cURL, Go, C#, Rust with reqwest). The
  Java, Kotlin and Dart clients read them only when the code asks
  (`ProxySelector.getDefault()`, `HttpClient.findProxyFromEnvironment`), and a
  raw socket does not use a proxy at all; for a limit that holds for every
  language, run Yon with Firecracker isolation and its egress setting.

## Consequences

- The codes are `TY1322` (route access and role types), `TY1323` (identity
  provider), `TY1324` (secrets), `TY1325` (text run as code and built
  queries) and `TY1326` (undeclared hosts).

- Every controller of every example, fixture, corpus project and `ty init`
  template now says `@Public` or another rule. An existing app must do the
  same before it builds. `ty migrate fix` writes `@Public` only when it turns
  an old file route (`server/routes/.../yon.<ext>` or `@Controller`) into a
  controller, because those routes were open to anyone before and the fix
  keeps what they did. It never adds a rule to an existing `@RequestRoute`
  controller: choosing who may call a route is a security decision.
- The website has a guide, "Who may call a route", with the identity provider
  in all eleven Yon languages. The snippet check signs in with a demo token
  and checks that a guarded route answers 401 without it.
- The annotation rule table has rows for `Public`, `SignedIn`, `Allow` and
  `IdentityProvider`. `#[allow(...)]` in Rust stays a Rust attribute; the
  Rust spelling of the Tachyon annotation is `#[Allow(...)]`.

## Amendment (2026-10-10): the guard refuses a Node.js that ignores the proxy switch

- Node.js sends `fetch` and `http.request` to the guard only from version
  24.5, when `NODE_USE_ENV_PROXY=1` is set. Node.js 23.5 ignored the switch
  and reached a private loopback port directly, so the guard failed open.
- While the guard is active, `ty` asks the JavaScript runtime what it is when
  the server starts, and keeps the answer. Every JavaScript handler start
  checks that answer first, so a handler added later is covered too.
- Bun, which reads the proxy variables itself, and Node.js 24.5 or newer are
  accepted. Any other answer, including an older Node.js, is refused with
  `TY2101` ("Required Yon JavaScript runtime 'node' is not usable."), so a
  handler never runs without the guard.

## Amendment (2026-10-18): every native host sends the policy and enforces `tac.outgoing.hosts`

- Section 4 said the desktop and mobile hosts keep their own built-in policy.
  In fact only the Windows and Linux hosts sent one, and it allowed
  `connect-src 'self' https:`, so page code in a native app could reach any
  host. The macOS, iOS and Android hosts sent no policy at all.
- Now every native host sends the policy that `ty bundle` builds from the
  same source as the server's: the macOS and iOS scheme handlers, the Android
  `WebResourceResponse` for each bundle file, and the Windows and Linux
  resource handlers. Its `connect-src` is the app's own bundle plus the hosts
  in `tac.outgoing.hosts`, with the same `https`, `wss`, `http` and `ws`
  sources as the server. With no list, a native page may connect only to its
  own bundle. Images and fonts may still load over `https:`.
- The Android host writes its bridge script into each page as an inline
  `<script>`. Its policy allows that one script by the SHA-256 hash of its
  text, which the build computes. The short start-up document that the host
  makes itself has its own policy (`default-src 'none'`, its one inline
  script allowed).
- A native build reads `tac.outgoing.hosts` and refuses an entry that is not
  a host name, such as one with a scheme or a port, with `TY1601`, because
  the hosts are written into the generated host source.
- Android no longer blanks the declared hosts: a request to a listed host
  over `https` goes to the network, and every other request outside the
  bundle still gets an empty answer. The app asks for the
  `android.permission.INTERNET` permission only when `tac.outgoing.hosts`
  lists a host.
- The Windows response header buffer is sized from the policy, so a long
  host list is never cut short.
- The build check (`TY1326`) is unchanged. The run-time half now holds on
  the web server and in every native host, as `host_check.rs` already told
  developers.

## Amendment (2026-10-19): native pages may frame their own documents

- The native policy said `frame-src 'none'`, while the server's policy lets a
  page frame documents from its own origin (`default-src 'self'`). Native
  pages now use `frame-src 'self'` on every host, so an `<iframe>` of a
  bundle document works as it does on the web. A frame from any other host
  is still refused, and the native bridges still answer only the main frame
  (ADR 0018).
- An `about:srcdoc` frame inherits its parent's policy, so an inline script
  in it never runs. A `srcdoc` frame that needs a script loads it from the
  bundle with `<script src>`. The Android host test's opaque frame probe
  does this, and its foreign frame probe now checks that the policy refuses
  the frame (`securitypolicyviolation` on `frame-src`).

## Amendment (2026-10-25): `connect-src` refusal runs in the apps

The 2026-10-18 amendment above made every native host send the policy. These
runs checked that a page fetch to a host outside `tac.outgoing.hosts` is
refused in the app and that the host receives no request
(`docs/SUPPORT_TIERS.md`):

- Linux (RL1, 2026-10-19, `scripts/linux/page-checks-native-test.mjs`, under
  Xvfb in an x86_64 Ubuntu 24.04 cloud container): refused while the Yon
  origin answered.
- macOS (RM1, 2026-10-24, `scripts/native/page-checks-macos-test.mjs`, macOS
  26.6.2): the page's `securitypolicyviolation` event named `connect-src`.
- iOS (RM2, 2026-10-24, `scripts/ios/page-checks-test.mjs`, iOS 26.5
  simulator): a `connect-src` violation was raised.
- Windows (SD1, 2026-10-24, `scripts/windows/page-checks-native-test.mjs`,
  inside WebView2 on a signed-in Windows 11 ARM64 desktop).

- Android (MA3, 2026-10-25, `scripts/android/page-checks-test.mjs`, the
  Android 15 emulator with System WebView 124): a fetch to a loopback host
  that `adb reverse` would have reached raised a `connect-src` violation,
  and the host saw no request.
