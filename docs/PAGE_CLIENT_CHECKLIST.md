# Checklist: add a page client for Tac services

A page client is a browser API that Tac services call in every language,
such as `Geolocation`, `Push` or `StorageSpace` (ADR 0065, ADR 0118,
ADR 0123). The page answers each call. A slow answer comes back later as a
built-in event. Follow this list in one branch; the paths are relative to
`crates/tachyon-core/src/` unless they start with `docs/` or `types/`.

Use `StorageSpace` (ADR 0123) as the smallest complete example: `git log
--stat` of its commit lists every file below.

## 1. Names and grants

- [ ] Client type name in `web_api_clients.rs` (`TYPE_NAMES`, and the array
      length). This also makes it a browser resource for every language.
- [ ] Field name (camelCase) in `events.rs` (the injectable names).
- [ ] If the API needs the user's consent: the grant in
      `native/browser_api_grants.rs` (`KNOWN`, sorted), and the error text in
      `native/site_tests.rs`. A grant that a native host checks also goes in
      `native/browser_api_permissions.rs` (`HOST_CHECKED`) and, for Android,
      `native/android_browser_apis.rs` (`ANSWERED`).
- [ ] Event models in `route_types/web_event_models.rs`. One table makes the
      types for every language. Use `Shape::Number`, `Text` or `Boolean`.
      Each row is `record(sender, name, fields)` in a group of rows (add a
      group if none fits). The sender is `WebEventSender::Page` when the
      event needs no grant, else `WebEventSender::Grant("<grant>")`; an app
      model of the same name wins, and `TY1376` stops the build only when a
      listener could hear the built-in event (ADR 0065).
- [ ] Go: add the client and each event to `ALIASED_TYPES` in `native/go.rs`.

## 2. The page

- [ ] Small entry in `template/tac-web.js`: a frozen object of calls, listed
      in `TacWebApis.apis`. Use `TacWebApis.later(...)` for slow calls: it
      checks the grant (`grant`, default the API name; `null` for no grant),
      the user gesture, and sends the answer or `BrowserCallFailed` to the
      caller.
- [ ] Large code goes in its own module that ships only when the manifest
      grants it (see `compiler/web_push.rs`, `compiler/page_files.rs`,
      `compiler/page_passkeys.rs`), not in `tac-web.js`. Add it to the page
      files before the service worker is built, so offline pages keep it.
- [ ] Failures that the app must tell apart get their own event with a
      `reason` word, as `PasskeyFailed` does (ADR 0126); `BrowserCallFailed`
      stays for programming errors.
- [ ] Size budget: `MOST_RUNTIME_BYTES` in `tac_web/web_api_tests.rs`. Raise
      it only with a reason in the ADR.
- [ ] JavaScript and TypeScript injection: `template/tac-client.js`
      (`ownerResources`), `types/tachyon-behaviors.d.ts` (the client
      interface, each event model, and the injected field).

## 3. One binding per language

Copy the closest existing client; each is 10 to 25 lines.

- [ ] Rust `native/web_apis/<name>.rs`, listed in `native/rust.rs`.
- [ ] Go `native/web_apis/<name>.go`, listed in `native/go.rs`.
- [ ] C# (and F#) `native/web_apis/<Name>.cs`, listed in `native/csharp_web_apis.rs`.
- [ ] Java `native/web_apis/<Name>.java`, listed in
      `native/java_web_services/java_web_apis.rs` (array length).
- [ ] Kotlin `native/web_apis/<Name>.kt`, listed in
      `native/kotlin_web_services.rs` and `client_tests/kotlin_unit_files.rs`.
- [ ] Swift `native/web_apis/<Name>.swift`, listed in `native/swift_web_services.rs`.
- [ ] Dart `tac_dart/<name>.dart`, listed in `tac_dart.rs`.
- [ ] Ruby `tac_ruby/<name>.rb`, listed in `tac_ruby.rs`; the type name in
      `annotation_layout/single_class/design_check/type_rule/yard_names.rs`.
- [ ] Lua: a table in `tac_lua/web_apis.lua`.
- [ ] C++: a class and a `using` line in `tac_cpp/tachyon/web_apis.hpp`; the
      list in the error text of `tac_cpp/cpp_service_graph.rs` and its test.
- [ ] Python `tac_python/web_apis/<name>.py`, listed in
      `tac_python/python_browser_apis.rs`.
- [ ] Run `cargo run -p cohesion-scan`. Long lists of `include_str!` lines
      look the same to the duplication check, so one more entry can push two
      lists over its limit. Then put the new entry in a short list of its
      own, as `StorageSpace` and `Passkeys` are in the Java, C# and Kotlin
      `LATER_*` lists.

## 4. Native shells

A native app has no browser for some APIs. Either the web view answers by
itself, or the host answers a `browserApi.<api>.<call>` host call:

- [ ] Apple: `native/browser_apis/AppleBrowserApis.swift` (plus a new Swift
      file listed in `native/apple_browser_apis.rs`).
- [ ] Android: `native/native_browser_apis_android.java`.
- [ ] Windows: `native/windows_browser_apis.c`, with checks in
      `native/windows_browser_api_fixture.c`.
- [ ] Linux: `native/linux_browser_apis.c` and the router in `native/linux.rs`,
      with checks in `native/linux_browser_api_tests.rs`.
- [ ] `native/browser_api_bridge.rs`: either the per-target `BRIDGED` list
      (the host replaces the page's answer) or `EVERY_HOST` (the page calls the
      host itself). Its test checks that every host source answers each call.
- [ ] A reach row per target in `native/target_support.json` (ADR 0102).

## 5. Tests

- [ ] Page runtime: a node driver in `tac_web/` that runs `tac-web.js` with
      stubs (granted, not granted, web, native host).
- [ ] One service test per compiled language (Rust, Go, C#, F#, Swift,
      Kotlin, Java, Dart, Ruby, Lua, C++): the service calls the client and
      hears the event. See `tac_web/storage_space_tests.rs`.
- [ ] Python in `tac_python/browser_api_tests.rs`; JavaScript in
      `compiler/web_api_tests.rs` (`a_javascript_service_needs_every_curated_browser_api`).
- [ ] Run `tachyon-ci --vms-only --filter '<filterset>' <branch>` when a shell
      changed.

## 6. Documents

- [ ] A new ADR, or an amendment; the number is the next free one at merge.
- [ ] `docs/adr/README.md`, `CHANGELOG.md`, and the `tac.browserApis` row of
      `docs/APPLICATION_CONFIGURATION.md` for a new grant.
