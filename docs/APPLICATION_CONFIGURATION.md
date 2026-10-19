# Application configuration

Tachyon splits configuration in two ways: by realm (Tac on the client, Yon on
the server) and by secrecy (public or private).

| File | Owner | Secrecy | What it holds |
| --- | --- | --- | --- |
| `client/manifest.json` | Tac | Public | The web app manifest. Its `tac` member is Tac's public configuration. The whole file goes to every visitor ([ADR 0034](adr/0034-tac-configuration-is-the-web-app-manifest.md)). |
| `server/application.json` | Yon only | Not secret | Application Configuration v3. The file is the `yon` namespace itself. Tachyon reads its members as `yon.*` paths. The file has no root key. |
| `.env` in the project root | Yon only | Private | Yon secrets. |

ADR means Architecture Decision Record.

Both kinds of files (JSON and `.env`) use bounded parsers. They add no new
dependency. The configuration parser itself adds no dependency for a file
format.

At startup, Yon also initializes the datasource and Redis resources that you
configure. For connection settings and repository injection, see
[Managed storage](MANAGED_STORAGE.md).

`yon.datasource.driver` accepts:

| Driver | `yon.datasource.url` | Credentials |
| --- | --- | --- |
| `postgresql` | `postgresql://host:5432/catalog` | `.env` username/password |
| `mysql`, `mariadb` | `mysql://host:3306/catalog` | `.env` username/password |
| `sqlite` | `sqlite::memory:` or `sqlite:///absolute/existing.db` | None |
| `sqlserver` | `sqlserver://host:1433/catalog` | `.env` username/password |
| `fylo` | `fylo:///absolute/root` | None; no TLS keys either |

The `fylo` driver ([ADR 0032](adr/0032-fylo-datasource.md)) has special rules:

- It takes only `driver` and `url`.
- Its root directory must already exist, and the path must be written in
  canonical form.
- It runs a local `fylo` subprocess. It does not connect to a database server.
- So `YON_DATASOURCE_USERNAME` and `YON_DATASOURCE_PASSWORD` do not apply to it.
- Its collections are created by the application with
  `dataSource.createCollection(name)`, not by this file. See
  [MANAGED_STORAGE](MANAGED_STORAGE.md#fylo) for names, documents, scroll and
  the queue.

## Files and paths

`client/manifest.json` is the web app manifest. A browser installs the app from
this file:

```json
{
  "name": "Catalog",
  "short_name": "Catalog",
  "start_url": "/",
  "display": "standalone",
  "tac": { "api": { "url": "/api" } }
}
```

`server/application.json` is Yon's server configuration. It holds no secrets:

```json
{
  "server": { "host": "127.0.0.1", "port": 8080 },
  "datasource": { "driver": "postgresql", "url": "postgresql://localhost:5432/catalog" },
  "catalog": { "page": { "size": 20 } }
}
```

`.env` holds secrets. The values below are example placeholders. They are never
real credentials:

```dotenv
YON_DATASOURCE_USERNAME=replace-me
YON_DATASOURCE_PASSWORD=replace-me
YON_AUTH_SIGNING_KEY=replace-me
```

Rules for `.env`:

- Do not commit `.env`.
- When it helps, commit a `.env.example` file without secrets.
- Tachyon loads only the exact `.env` file in the project root.
- A production secret manager may create that file. The file must pass the
  same checks as other private configuration inputs: it must be a regular file,
  and Tachyon does not follow links to it.

Each underscore in a name starts another nested key:

| Application name | Canonical path |
| --- | --- |
| TAC_API_URL | tac.api.url |
| YON_SERVER_PORT | yon.server.port |
| YON_CATALOG_PAGE_SIZE | yon.catalog.page.size |
| YON_DATASOURCE_PASSWORD | yon.datasource.password |
| YON_AUTH_SIGNING_KEY | yon.auth.signing.key |

Path rules:

- A JSON path segment starts with a lowercase ASCII letter. The rest is
  lowercase letters or digits.
- Underscores, dots and empty segments are never literal JSON keys.
- Arrays are values. Environment names cannot index into an array.
- Numbers and booleans stay typed JSON values. Secrets are literal strings.
- Tachyon does not silently pick one value when paths conflict. These cases
  fail:
  - one path is a scalar and another path needs an object at the same place;
  - a secret tries to overwrite a value from the JSON configuration.

## Field reference

### `client/manifest.json`

This file is a standard web app manifest:

- A browser reads its standard members to install the application.
- Tachyon reads a few of them for native builds.
- The `tac` member is the whole Tac configuration (ADR 0052). There is no
  `tac.config.js` and no `tachyon.json`; a build refuses both and says where
  each value moved.

The whole file is public. Tachyon publishes it at `/manifest.webmanifest`.
Never put a secret in it, including in `tac.application`.

| Member | Meaning | Read by |
| --- | --- | --- |
| `name` | The application's name. A native build uses it to name the app and the executable (it uses `short_name` when `name` is absent). 1 to 64 printable characters. | browser, native |
| `short_name` | The label that a launcher shows. | browser, native |
| `start_url` | The route where the application opens, and the entry route of a native build. It must be a canonical local route. | browser, native |
| `icons` | `[{ "src", "sizes" }]`, with local bundle paths. A native build uses the largest raster icon. A manifest with only SVG icons gets no native icon. | browser, native |
| `theme_color` | The color of the browser's interface around the page. Tachyon publishes it as written. When it is a text value that is not blank, every page head also gets `<meta name="theme-color" content="…">` with that value, HTML-escaped, so the color applies before installation. A native build writes the same tag into its pages. On Windows 11 the app also colors its title bar with it when it is written as `#rgb` or `#rrggbb`, and picks black or white title text by contrast. A Linux app does the same with a GTK header bar, which then replaces the window manager's title bar. Other forms, Windows 10, and the other native windows and status bars do not use it yet, and the build prints a note. A number or another JSON value writes no tag. The manifest has one theme color: there is no light and dark pair (ADR 0052 amendment). | browser, build, native |
| `display` | How the installed application shows: `fullscreen`, `standalone`, `minimal-ui` or `browser`. Tachyon publishes it as written; any other value stops the build (`TY1601`). A Windows app with `fullscreen` opens borderless and covers the monitor; the page can leave full screen only with the `fullscreen` window control. A Linux app with `fullscreen` starts full screen. The other values open a standard window. | browser, native |
| `background_color`, … | Standard members for browser install. Tachyon publishes them as written. | browser |
| `tac` | Tac's public configuration (ADR 0034). It binds with `TAC_` names and `tac.` paths. Keys are lowercase. Names that look like secrets are not allowed. The Tachyon members below are read by the build and are not service configuration. | Tac services |
| `tac.application.id` | Bundle identifier in reverse-DNS form, such as `com.example.catalog`. Without it, the id is derived from the project folder. | native |
| `tac.application.version` | The version people see, such as `1.4.0` (1.0.0 unless set): up to 64 letters, digits, `.`, `-` or `+`. It is `CFBundleShortVersionString` on macOS and iOS, `versionName` on Android, the file and product version of the Windows executable and its manifest, and `X-Tachyon-Version` in the Linux desktop entry (the desktop entry format has no version key) and the AppStream release and `.deb` version of a Linux release, which needs a version that starts with a digit. macOS, iOS and Windows need one to three numbers separated by dots, such as `1.4.0`, and Windows needs each number to be at most 65535; another version stops a build for those targets (`TY1601`). | native |
| `tac.application.build` | The build number, a whole number from 1 (1 unless set). Raise it for every build you hand out. It is `CFBundleVersion` on macOS and iOS, `versionCode` on Android (at most 2100000000), the fourth number of the Windows file version (at most 65535, so `1.4.0` with build 12 is `1.4.0.12`), and `X-Tachyon-Build` on Linux, where a release also uses it as the `.deb` revision (`1.4.0-12`). | native |
| `tac.application.window` | `width`/`height`: 1024 by 768 unless set, each from 64 to 16384. macOS, Windows and Linux open the window's content (the page, without the title bar or header bar) at this size; Windows counts device-independent pixels, so the window keeps its size on a high-DPI screen. `minWidth`/`minHeight`: the smallest content size the user can drag the window to on macOS, Windows and Linux. `controls`: the window controls that a page may use (`close`, `fullscreen`, `maximize`, `minimize`, `resize`, `title`). A page may use none unless they are listed. On iOS only `title` applies (the scene title); the other controls answer `granted` false there and the build prints a note. `titleBar`: `standard` (default) or `integrated`; on macOS an integrated title bar lets the page reach the top edge under the real window buttons, sets `--tac-titlebar-height`, `--tac-titlebar-leading-inset` and `--tac-titlebar-trailing-inset`, and moves the window from elements marked `data-tac-drag-region` (ADR 0098). On Linux the page also reaches the top edge, under GTK's own window buttons, with the same three properties and drag regions; the menu bar moves into a main menu button beside the window buttons, and `theme_color` colors no header bar. Windows, iOS and Android keep the standard title bar and print a note. `titleBarButtonInset`: `{ "leading", "top" }` in points (0 to 200), only with `integrated`; on Linux `leading` is measured from the window edge where each group of buttons sits. | native |
| `tac.application.permissions` | The reason the app gives when it asks the user for a browser API, such as `{"geolocation": "Shows stores near you", "notifications": "Order updates"}`. Only `geolocation` and `notifications` take a reason, each must also be in `tac.browserApis`, and each reason is 1 to 200 characters of plain text. A native build for iOS, macOS or Android that grants `geolocation` without a reason, or an Android build that grants `notifications` without one, stops with `TY1601`. iOS and macOS show it from `Info.plist`; Android uses it for its permission rationale (ADR 0065 amendment). | build, native |
| `tac.application.minimumSystem` | The oldest system each native app runs on: `ios` and `macos` as versions such as `"18.2"`, `android` as an API level such as `28`, `windows` as the WebView2 Runtime version such as `"119.0"`, and `linux` as the WebKitGTK version such as `"2.46"`. Without it, each app uses the host's own floor (iOS 17.0, macOS 13.0, API level 26, WebView2 86.0, WebKitGTK 2.40), raised to what the bundled services need: Dart, Kotlin and Java services that run as WebAssembly with garbage collection need iOS 18.2, macOS 15.2, WebView2 119.0 or WebKitGTK 2.46, and the build prints one note. A value below either floor stops the build (`TY1601`) and names the languages. Android keeps its API level, because Google Play updates the web view apart from Android, and the note names Android System WebView 119. The Windows and Linux apps check the installed web view at startup and show a readable message instead of a blank window (ADR 0033 amendment). | native |
| `tac.application.links` | The links that open the app: `{"schemes": ["myapp"], "hosts": ["example.com"]}`. `schemes` lists up to 16 lowercase URL schemes (a letter, then letters, digits, `+`, `-` or `.`; not `http`, `https`, `mailto`, `file` or a scheme Tachyon uses). `hosts` lists up to 16 lowercase host names for verified web links. Anything else stops the build (`TY1601`). A link with a listed scheme, or an `https` link to a listed host, opens the app at the route of its path with its query; an unknown route opens `start_url`. macOS and iOS claim the schemes in `Info.plist`, Android in intent filters (the hosts with `autoVerify`), Linux in the `.desktop` file, and Windows under `HKCU\Software\Classes` when the app starts (a `--release` MSIX package declares them instead). A link that arrives while the app runs goes to the open window. With `hosts`, the web build writes `/.well-known/apple-app-site-association` and `/.well-known/assetlinks.json` from the variables in "Links to the app" below. Each native build prints a note for what its target does not do yet (ADR 0106). | build, native |
| `tac.nativeCapabilities` | The operating system features that native services may use, such as `["filePicker"]`. Each name may appear once; `filePicker` is the only name today. Without the grant, the file picker answers `denied` before any dialog opens. The grant does not replace the user's consent or the platform's signing entitlements (ADR 0089). | build, native |
| `tac.origin` | The site's public address, such as `https://example.com`, without a path. It turns on the built-in `sitemap.xml` and `robots.txt`, and a `canonical` or `image` that starts with `/` is joined to it. | build |
| `tac.scripts`, `tac.styles` | Up to 8 local paths each, such as `/shared/app.js`, linked into every page. Each script is a deferred `type="module"` script after the Tac runtime. | build |
| `tac.blockingScripts` | Up to 8 local paths, such as `/shared/scripts/ThemeDefault.js`, linked into every page as classic `<script src>` tags before the Tac runtime (`/.tachyon/tac-client.js`). Each one runs before the first paint, for example to apply a saved theme, and the page waits for it: keep it small, with no `import` or `export` (ADR 0052 amendment). | build |
| `tac.scriptSources` | For each third-party script in `tac.scripts`, its `package`, `version`, `license` and pinned `sha256`. The file stays in `client/shared/` and is listed in both settings (`TY1365`); bytes that do not match the pinned SHA-256 fail with `TY1403`. | build |
| `tac.styleSources` | The kind of each style in `tac.styles`: `{"kind": "foundation"}` for the project's own foundation styles, or `{"kind": "vendor", ...}` with the same `package`, `version`, `license` and `sha256` as a script source. | build |
| `tac.browserApis` | The browser APIs that need the user's consent and that services may use: any of `clipboard`, `share`, `notifications`, `geolocation` and `preferenceStrings`, each once. Any other name stops the build (`TY1601`); the page refuses an API that is not listed (ADR 0065). | build, browser |
| `tac.elements` | The components that the build also exports as custom elements, such as `["shop-counter"]`. A name without a hyphen, or a name that is not a component, stops the build (`TY1358`, ADR 0086). | build |
| `tac.application.menus` | The native File, View and Help menu items. An item's action names a client Rust application service, its method and an optional event model (`TY1370`, ADR 0097). | native |
| `tac.python` | `{"runtime": "pyodide-314.0.7", "profile": "offline"}` turns on Python services on the web. Any other value stops the build. `ty init` and `ty add language client py` write it (ADR 0093). | build |
| `tac.go.compiler` | `"tinygo"` or `"go"`: the compiler for Go web services. Without it, a release build tries TinyGo first and falls back to Go. | build |
| `tac.dotnet.aot` | `true` compiles C# and F# web services ahead of time in a release build: faster for heavy computing, but a larger download. | build |
| `tac.budgets.script`, `tac.budgets.webassembly`, `tac.budgets.check` | The size budget of every route in kilobytes: 200 for scripts and 3584 for WebAssembly unless set. Unless `script` is set, a route that loads more than one web runtime gets a measured allowance for each runtime after the largest one (ADR 0100). `check: false` turns the budget check (`TY1337`) off. | build |
| `tac.cache` | Up to 64 `{ "path", "policy" }` rules for the service worker; `policy` is `cache-first`, `network-first` or `no-store`, and `*` in `path` matches the rest of a path. A native app has no service worker, so its pages apply the same rules to Yon reads at `tac.yon.origin` themselves and keep them in Cache Storage (`tachyon-yon-reads`), which macOS, Windows and Linux keep on disk. On iOS the kept reads last only while the app runs (not yet proven by a clean rerun), and on Android a read kept just before the system stops the app may be lost. | build |
| `tac.metadata` | `defaults` and `routes`: page head values (`title`, `description`, `canonical`, `image`, `siteName`, `lang`, `noindex`). `routes` is keyed by route, such as `/docs/_topic`; each page gets its route entry over the defaults. | build |
| `tac.sitemap` | For a page route with one path variable, the values to list: `{ "data": "/shared/data/items.json", "values": "/items", "field": "id" }`. `values` is a JSON pointer to a list; `field` picks a member of each listed object. | build |
| `tac.outgoing.hosts` | The hosts that page code may send requests to, such as `["api.example.com", "*.cdn.example.com"]`: names without scheme or port. The build refuses a request to another host (`TY1326`), and the page's `connect-src` allows only these hosts, on the web server and in every native host (ADR 0068). A native build refuses an entry that is not a host name (`TY1601`), and an Android build asks for network access only when the list names a host. | build, browser, native |
| `tac.yon.origin` | The origin of the Yon server that the pages call, such as `https://api.example.com`: `https://`, a lowercase host and an optional port, with no path, trailing slash or user info. `http://` is allowed only for a local Yon (`localhost`, a name under `.localhost`, or a loopback IP address, such as `http://localhost:8080`). Any other value stops the build (`TY1601`). The build writes `<meta name="tachyon-yon-origin">` into every page, and Tac Client exchanges, `TacBrowserHttp` and the event bus (streams, topic streams and sockets) send route paths such as `/api/notes` to this origin over CORS, without cookies. The page's `connect-src` also allows it and its `wss://` form, on the web server and in every native host. A native app needs it to reach Yon: without it, a native build with `client/remotes` stops (`TY1601`) and one with Yon routes prints an offline-only note. Linked services send their `HTTPClient` requests to this origin too: Swift on macOS and iOS, Kotlin and Rust on Android, Rust and C# on Windows, and Rust on Linux. A linked Rust `HTTPClient` on macOS and iOS still answers `HTTP_MISSING`, because those hosts do not run Rust tasks yet. On Android a local `http` origin may use cleartext to that one host only. List the app origin in `server.origins` (ADR 0103). | build, browser, native |
| `tac.cohesion.check` | `false` turns off the cohesion check (`TY1314`, ADR 0063) for Tac services. Use it only while you migrate. | build |
| `tac.duplication.check` | `false` turns off the duplication check (`TY1315`, ADR 0064) for Tac code. Use it only while you migrate. | build |
| `tac.implicitmembers.suggest` | `true` turns on suggestions (`TY1362`) to drop a receiver such as `this.` that the language does not need before a class member. Off by default. Keep a receiver when a local name hides the member, inside a closure, and in a Dart constructor parameter such as `this.name`. This is a build setting, not service configuration. | build, migrate |
| `tac.dependencies.check`, `tac.names.check`, `tac.signatures.check`, `tac.constants.check`, `tac.visibility.check`, `tac.types.check` | `false` turns off one design rule (`TY1327` to `TY1330`, ADR 0069; `TY1333`, ADR 0071; `TY1356`, ADR 0079, off until set to `true`) for Tac code: dependencies point down, plain-English names, small signatures, named values. Use them only while you migrate. | build |
| `tac.tests.check` | `false` turns off the test-case rule (`TY1339`) for Tac code. Use it only while you migrate. | build |
| `tac.accessibility.check` | `false` turns off the accessible-view check (`TY1336`, ADR 0074). Use it only while you migrate. | build |
| `tac.simplify.suggest` | `false` hides the simpler-form suggestions for Tac code. | build |
| `tac.names.words` | Words of your domain, such as a brand or a product name, that Tac names may use (`TY1328`, ADR 0069), for example `["fylo"]`. Lowercase. | build |

### Each member on each target

Each member above is honored, run owed, adapted, not applied, refused or not
applicable on each of the six targets (Web, macOS, Windows, Linux, iOS,
Android). Run owed means the target supports the member in code, but the run
that proves it on that target has not happened yet; the table names that run.
Not applied means the target does not use the member yet. A build prints one
`Note:` line for each declared member that its target adapts, does not apply,
does not use, or has not run yet, and stops with a TY code for each declared
member that its target refuses.
It prints nothing for a member that the manifest does not declare. Run
`ty bundle --explain-targets` (optionally with `--target ios` or a comma list)
to print the whole list for your project; each declared member is marked
`(declared)`. The list is one table,
`crates/tachyon-core/src/native/target_support.json` (ADR 0102), and the
website shows it as one tab per target.

### Signing a release package

`ty bundle --target android --release` builds a signed APK and Android App
Bundle (`.aab`) next to each other in `dist/android/<Name>/`, and
`ty bundle --target macos --release` builds a signed, notarized
`dist/macos/<Name>.dmg`, and `ty bundle --target ios --release` builds
`dist/ios/<Name>.ipa` for App Store Connect (ADR 0104).
`ty bundle --target linux --release` builds an AppImage and a `.deb` in
`dist/linux/`; they use the system's GTK 4 and WebKitGTK 6, and their version
is `tac.application.version`, which must then start with a digit.
`ty bundle --target windows --release` builds a signed MSIX package,
`dist/windows/<Name>/<Name>.msix`, next to the app folder it was packed from.
Signing material is never part
of `client/manifest.json`. `ty` reads it from environment variables of the
build machine and from its keychain.

Android:

| Variable | Value |
| --- | --- |
| `TAC_ANDROID_KEYSTORE` | The keystore file (`.jks` or `.p12`). A relative path is read from the directory where `ty` runs. |
| `TAC_ANDROID_KEYSTORE_PASSWORD` | The keystore password. |
| `TAC_ANDROID_KEYSTORE_ALIAS` | The alias of the upload key in the keystore. |
| `TAC_ANDROID_KEYSTORE_KEY_PASSWORD` | Optional. The key's own password; the keystore password when it is not set. |

A release build stops with `TY1605` before Gradle runs when one of the first
three is missing or the keystore is not a file. The staged Gradle project reads
the same variables, so it also signs a release built outside `ty`.

macOS:

| Variable | Value |
| --- | --- |
| `TAC_APPLE_TEAM_ID` | The Apple Developer team ID, 10 capital letters and digits. `ty` signs with the keychain's Developer ID Application certificate for this team. |
| `TAC_APPLE_NOTARY_PROFILE` | The name of a `notarytool` keychain profile, made once with `xcrun notarytool store-credentials`. |
| `TAC_MACOS_SIGNING_IDENTITY` | Optional. The certificate to sign with, by name or SHA-1 hash, when the keychain holds more than one for the team. `-` signs ad hoc and skips notarization: the `.dmg` then opens only on the build Mac. |

The app and its helpers are signed with the hardened runtime. Their
entitlements come from the manifest: a `geolocation` grant in
`tac.browserApis` adds `com.apple.security.personal-information.location`.
The `.dmg` is signed, notarized, stapled and checked with `spctl`. A release
build stops with `TY1605` before anything is built when a variable is missing
or the team ID is malformed, and before signing when the keychain holds no
certificate for the team, or several.

Linux packages are not signed by `ty`. Two optional variables shape them:

| Variable | Value |
| --- | --- |
| `TAC_LINUX_MAINTAINER` | The `.deb` `Maintainer` field, such as `Ada Lovelace <ada@example.com>`. The app's name when not set. |
| `TAC_APPIMAGE_RUNTIME` | A local AppImage type 2 runtime file. Without it, `ty` downloads the runtime it pins once into its runtime cache and checks its SHA-256. |

iOS builds for a physical iPhone or iPad with `--device` (a development or ad
hoc profile; `dist/ios/<Name>.app` and `<Name>.ipa`) or for App Store Connect
and TestFlight with `--release` (an App Store profile; `<Name>.ipa`). Both run
on a Mac with Xcode and read `TAC_APPLE_TEAM_ID`, shared with macOS, and:

| Variable | Value |
| --- | --- |
| `TAC_IOS_PROVISIONING_PROFILE` | The `.mobileprovision` file for `tac.application.id`. A relative path is read from the directory where `ty` runs. |
| `TAC_IOS_SIGNING_IDENTITY` | Optional. A certificate name or SHA-1 from the keychain. Without it, `ty` signs with the one keychain certificate that the profile names. |

Both builds stop with `TY1605` before anything is built when a variable is
missing or the profile is not a file, and before compiling when the profile
belongs to another team, does not cover the app ID, or is the wrong kind for
the build. A `--release` build also needs an opaque PNG or JPEG icon in
`icons`, which `ty` scales to 1024x1024 for the asset catalog (`TY1601`
otherwise). When `tac.application.links` lists `hosts`, both builds sign them
as associated domains, so the profile's app ID needs the Associated Domains
capability (`TY1605` otherwise). Uploading the `.ipa` (with Transporter or
`xcrun altool`) is up to you.

Windows:

The package is signed with one certificate, from a file or from the
certificate store.

| Variable | Value |
| --- | --- |
| `TAC_WINDOWS_CERTIFICATE` | The code-signing certificate as a `.pfx` file. A relative path is read from the directory where `ty` runs. |
| `TAC_WINDOWS_CERTIFICATE_PASSWORD` | Optional. The `.pfx` file's password. |
| `TAC_WINDOWS_CERTIFICATE_THUMBPRINT` | Instead of a file: the SHA-1 thumbprint of a certificate in the current user's certificate store, such as one on a hardware token. |
| `TAC_WINDOWS_CERTIFICATE_PUBLISHER` | The certificate's subject, exactly as the certificate states it, such as `CN=Contoso Ltd, O=Contoso Ltd, C=US`. It becomes the package publisher, which Windows requires to match the certificate. |
| `TAC_WINDOWS_CERTIFICATE_TIMESTAMP_URL` | Optional. An `http://` or `https://` timestamp server, so the signature stays valid after the certificate expires. |

A Windows release build stops with `TY1605` before the host is compiled when
the publisher is missing, when neither or both of the certificate file and
thumbprint are set, or when the file does not exist. The package name is
`tac.application.id`, the version is `tac.application.version` with
`tac.application.build` as its fourth number, and the tiles come from the PNG
`icons`. Build it from a Visual Studio developer prompt, which puts the
Windows SDK's `makeappx.exe` and `signtool.exe` on `PATH`. To install a package
signed with a self-signed test certificate, the certificate must first be
trusted on that machine (for example in `Cert:\LocalMachine\TrustedPeople`).

### Links to the app

`tac.application.links` declares the links that open a native app (ADR 0106).
Verified web links also need the app's signing identity, which is never part
of `client/manifest.json`. `ty` reads it from these environment variables:

| Variable | Value |
| --- | --- |
| `TAC_APPLE_TEAM_ID` | Your Apple Developer team ID: 10 uppercase letters and digits. With `hosts` listed, the web build writes `/.well-known/apple-app-site-association` for the app ID `<team>.<tac.application.id>`, and an iOS build writes `TachyonHost.entitlements` with an `applinks:` entry per host and signs the app with it. Universal links open the app only from a signed device build. |
| `TAC_ANDROID_CERT_SHA256` | The SHA-256 fingerprints of your Android signing certificates, separated by commas, each 32 hexadecimal pairs joined by `:` (`keytool -list -v` prints them). With `hosts` and `tac.origin` set, the web build writes `/.well-known/assetlinks.json` for the app's package name. |

A malformed value stops the build with `TY1601`. Without them, the web build
prints a note and writes nothing for that platform. Each host must serve the
file at its own `https` origin. Tachyon's server sends
`apple-app-site-association` as `application/octet-stream`; when another
server or CDN serves the web build, send it as `application/json`.

### `server/application.json`

This file is Yon's configuration. The file itself is the `yon` namespace: its
members are `yon.*` paths, with no root key. Tachyon reads `server`,
`datasource`, `redis`, `topics`, `outgoing`, `cohesion`, `duplication`, `dependencies`, `names`, `signatures`, `constants`, `visibility`, `types`, `tests`, `simplify` and `implicitmembers`. Every other member belongs to the application. A
profile file has the same shape. `server` is a closed set: it holds only
`host`, `port`, `hosts`, `origins`, `proxies`, `environment` and `maxbody`,
and any other member stops the server with "yon.server supports only host,
port, hosts, origins, proxies, environment and maxbody."

| Path | Meaning |
| --- | --- |
| `server.host` | The address that `ty serve` and `ty start` bind. `127.0.0.1` unless set. `localhost` or a name under `.localhost`, such as `shop.localhost`, binds `127.0.0.1` and the ready line prints the name (ADR 0094). An address that is not loopback also needs `--allow-non-loopback`. `--host` overrides it. |
| `server.hosts` | Extra `Host` names a loopback server answers, such as `["preview.example.com", "*.tunnel.example"]`: names without scheme, port or path. Localhost names, IP addresses and the `tac.origin` host are always answered; other names get `403` with `TY1366`. With `--allow-non-loopback`, the `Host` check runs only when this list is set (ADR 0094). |
| `server.origins` | The app origins that may call Yon from another origin, such as `["tachyon-app://bundle", "https://appassets.tachyon.local", "https://app.example.com"]`: each written exactly as `scheme://host[:port]`, with no path. `*`, `null`, a path, a trailing slash, user info and an uppercase host are refused with `TY2010`. A request whose `Origin` is listed gets CORS answers: a preflight gets `204` before any route, and every response gets `Access-Control-Allow-Origin` with that origin, never `Access-Control-Allow-Credentials`. When the list is set, every response carries `Vary: Origin`. A listed origin may also open a socket. Other origins get no CORS headers, and the `Host` check still runs first. `ty init` and `ty add target` write the fixed origins of native targets: macOS and iOS `tachyon-app://bundle`, Android `https://appassets.tachyon.local`, Windows `https://tachyon.local` and Linux `tachyon://app` (ADR 0103). |
| `server.proxies` | The reverse proxies whose forwarding headers count, as IP addresses or CIDR ranges such as `["10.0.0.0/8", "192.0.2.10"]`; a host name is refused. Empty by default, so `ty` removes `Forwarded`, `X-Forwarded-*` and `X-Real-IP` from every other peer and writes the real client address. From a listed proxy it keeps them, appends its own hop and sets `X-Real-IP` to the nearest address that is not a listed proxy (ADR 0096). |
| `server.environment` | The environment variables of `ty` that handler processes may see, beyond the baseline (`PATH`, `LANG`, `LC_ALL` and `TMPDIR` on Unix). Each entry is a name, such as `HOME`, or a prefix pattern that ends in `*`, such as `HD_*` or `AWS_*`. Empty by default. `*` alone is refused, and so is any entry that names or covers `ty`'s own variables (`TAC_*`, `TACHYON_*`, `YON_*`, `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY`, `NODE_USE_ENV_PROXY`), with `TY2010`. Every Yon language, Gateway command and relay program gets the same list. See [Environment variables for handlers](#environment-variables-for-handlers). |
| `server.port` | Port, 0 to 65535. `8080` unless set. 0 picks a free port. `--port` overrides it. |
| `datasource.driver` | `postgresql`, `mysql`, `mariadb`, `sqlserver`, `sqlite` or `fylo`. |
| `datasource.url` | The database location, without credentials: `postgresql://db:5432/catalog`, `sqlite:///var/lib/catalog.db`, `sqlite::memory:`, or an absolute FYLO root. |
| `datasource.username` | The database user. Usually it is `YON_DATASOURCE_USERNAME` in `.env`, next to `YON_DATASOURCE_PASSWORD`. Only `.env` may hold the password. SQLite and FYLO take neither. |
| `datasource.tls.ca` | Absolute path of a CA (certificate authority) certificate to trust. |
| `redis.url` | `redis://`, or `rediss://` for TLS. |
| `redis.username` | The Redis user. Usually it is `YON_REDIS_USERNAME` in `.env`, next to `YON_REDIS_PASSWORD`. Only `.env` may hold the password. |
| `topics.directory` | The folder of the topic logs (ADR 0061). `data/topics` unless set. A plain path inside the project, such as `storage/events`, or an absolute path with no symbolic link in it. A path with `..`, a path inside `.tachyon`, and a folder that a symbolic link leads out of the project stop the server (`TY1303`). |
| `outgoing.hosts` | The hosts that Yon code may send requests to (`yon.outgoing.hosts`), such as `["api.example.com"]`: names without scheme or port, or `*.example.com`. The build refuses a request to another host (`TY1326`), and at run time handler processes send their requests through a local guard that answers 403 to other hosts. With no list, every outgoing request is refused (ADR 0068). |
| `cohesion.check` | `false` turns off the cohesion check (`TY1314`, ADR 0063) for Yon services and controllers. Use it only while you migrate. |
| `duplication.check` | `false` turns off the duplication check (`TY1315`, ADR 0064) for Yon code. Use it only while you migrate. |
| `implicitmembers.suggest` | `true` turns on suggestions (`TY1362`) to drop a receiver such as `this.` that the language does not need before a class member in Yon code. Off by default. Keep a receiver when a local name hides the member or the code is in a nested scope. |
| `dependencies.check`, `names.check`, `signatures.check`, `constants.check`, `visibility.check`, `types.check` | `false` turns off one design rule (`TY1327` to `TY1330`, ADR 0069; `TY1333`, ADR 0071; `TY1356`, ADR 0079, off until set to `true`) for Yon code. Use them only while you migrate. |
| `tests.check` | `false` turns off the test-case rule (`TY1339`) for Yon code. Use it only while you migrate. |
| `simplify.suggest` | `false` hides the simpler-form suggestions for Yon code. |
| `names.words` | Words of your domain that Yon names may use (`TY1328`, ADR 0069), for example `["fylo"]`. Lowercase. |
| anything else | The application's own settings. For example, `catalog.page.size` is `yon.catalog.page.size` and `YON_CATALOG_PAGE_SIZE`. |

Keys use lowercase letters and digits. A file holds at most 128 keys, 16
levels and 64 KiB.

## Secrets and validation

### Which secrets are allowed

- `.env` accepts only YON_ secret names. Tachyon rejects TAC_ secrets.
- Treat everything that goes to a browser or a native client as public.
- Application JSON rejects recognized credential keys and URLs that contain
  credentials. The recognized keys are: password, passwd, secret/secrets,
  token/tokens, credential/credentials, privatekey, signingkey, apikey,
  accesskey, clientsecret and private/signing/api/access.key.
- Tachyon cannot recognize every unusual secret name by its spelling.
  Developers must still put those values in `.env`.

### How Tachyon reads `.env`

The dotenv reader accepts:

- bounded literal assignments;
- blank lines;
- full-line comments that start with #.

Parsing rules:

- It trims whitespace around values.
- It removes matching single or double quotes. It does not process escapes.
- An unquoted # is literal data. It does not start an inline comment.
- It does not run a shell, run an export command, interpolate variables, read
  multiline secrets or change the process environment.
- JSON environment placeholders are no longer supported.
- Reserved supervisor names and deployment-policy names cannot be secret
  assignments.

### Environment variables for handlers

`ty` clears the environment of every handler process. A handler sees only the
baseline and the names that `server.environment` lists, so configuration, not
code, decides what a handler can read. Use it for cloud SDKs and tools that
read their credentials from the environment:

```json
{
  "server": {
    "environment": ["HOME", "AWS_*", "HD_*"]
  }
}
```

With this list, `AWS_ACCESS_KEY_ID` and `HD_REGION` reach handlers in every
Yon language as ordinary environment variables (`process.env`, `os.environ`,
`os.Getenv`, `System.getenv` and so on). `OTHER_SECRET` does not. Values stay
in the environment of `ty`: they are never written to the configuration, to
logs or to diagnostics. A refused entry names the entry, never a value.
Settings that belong to the application itself, such as a database password,
still go in `.env` as `YON_` names and reach handlers through the
configuration snapshot.

### Request body limit

A Yon route reads at most 1 MiB of request body. `server.maxbody` raises
that limit for every route, up to 12 MiB (12582912 bytes), for example for
mail or file uploads:

```json
{
  "server": {
    "maxbody": 12582912
  }
}
```

A larger body gets `413 Payload Too Large`. Tachyon reads up to 16 MiB of
the body before it answers, so a client that is still sending sees the 413
instead of a broken connection. The 12 MiB ceiling leaves room in the 16 MiB
message that carries a request to its handler (ADR 0003).

### How Tachyon validates JSON

JSON files reject:

- duplicate keys;
- a `yon` or `tac` root key;
- comments and trailing commas;
- invalid key segments;
- secret fields;
- too large a size, depth or key count.

Inputs are limited to 64 KiB. The resolved transport is limited to 16 KiB.
Diagnostics and Debug output leave out values. Never log or return the private
resolved snapshot.

## Profiles and precedence

- Yon selects one `server/application-<profile>.json` file through the
  `YON_PROFILE` variable of the parent process.
- The profile name is bounded and safe to use in a path.
- If you explicitly select a profile file and it is missing, Yon fails.
- Objects merge recursively. Arrays replace each other.

Endpoint precedence, from highest to lowest:

1. Explicit CLI --host / --port.
2. Selected profile value.
3. Base server/application.json.
4. Existing command default.

Other rules:

- Application variables in the parent process, such as YON_PORT and
  YON_DATASOURCE_PASSWORD, do not override file settings. They do not supply
  secrets either.
- The operator's environment policy for runtime and isolation is separate. It
  does not change.
- Exposure on an address that is not loopback still requires
  --allow-non-loopback.
- The behavior context activates WhenProfile annotations explicitly. Selecting a
  file does not automatically switch the DI (dependency injection) container.

## Public Tac and private Yon snapshots

### Tac: public

Tac's configuration is the `tac` member of `client/manifest.json`.

- Discovery validates it with the same key rules as `server/application.json`.
  It also rejects names that look like secrets, in the same way.
- The build publishes the manifest, including the `tac` member, at
  `/manifest.webmanifest`. Every page links to it, so a browser can install the
  app from it.
- Web bundles and staged native bundles carry the same file.
- The manifest's own members (`name`, `icons`, `start_url`) belong to the
  browser. They are not configuration. Only `tac` binds.
- The Tachyon members of `tac` in the field reference above (`origin`,
  `application`, `scripts`, `blockingScripts`, `styles`, `outgoing`, `budgets`, the `check` and
  `suggest` switches, `python`, `go`, `dotnet` and the others) are build
  settings. Tachyon removes them before services bind, so `Settings` and
  `Setting` never see them.
- No profile overlays it.
- Compilation never opens `.env` or a Yon profile. It never publishes the `yon`
  namespace.
- A build succeeds without production credentials.
- Native packaging is not secret storage.

### Yon: private

`server/application.json` and its profiles configure Yon only. If one of them
still has a `tac` section, the build stops with a message that names the
manifest. `ty migrate` also reports it.

### Loading public settings in a browser service

A browser service can load public JSON through a declarative client and then
bind it explicitly. Put this role in `client/remotes/SettingsClient.js`:

```javascript
@HTTPClient
class SettingsClient {
  @GetRequest("/manifest.webmanifest")
  async load() {}
}
```

- `NeedsAll` injects a `settingsClient` field into the browser
  service (`client/services/SettingsService.js`).
- `await this.settingsClient.load()` returns the published manifest.
- The manifest binds as it is, for
  `Behaviors.properties(PublicSettings, application)` or
  `Behaviors.values(PublicValues, application)`.
- Native services keep a platform loader that they own explicitly, or a public
  snapshot that is supplied to them.
- Tachyon adds no implicit synchronous filesystem or network access. It adds no
  global Tac dependency container.

Path and name rules:

- Tac Setting paths must start with tac.
- Yon paths must start with yon.
- Settings keeps its prefixed name spelling. It resolves each
  underscore into the canonical nested path.
- Settings takes only the prefix (and optional `names` renames). The class's
  instance fields are the settings: the declared type and initial value of a
  field give its type and default, and `@Min`/`@Max` on the field give its
  range. A field without an initial value is required. The environment name is
  the field name in capitals with underscores, so `pageSize` reads
  `YON_CATALOG_PAGE_SIZE`. Rust and Go fields have no initial values, so their
  defaults go in `"defaults":{...}`.
- Flat in-memory property maps that the caller supplies explicitly still work
  with adapters. They are not a way to write server/application.json.

### The Yon snapshot at runtime

- Server startup reads the configuration and `.env` once, into a private
  snapshot that cannot change.
- Handlers, middleware and workers receive the snapshot as the reserved
  supervisor output `YON_APPLICATION_JSON`.
- A value from the parent process cannot replace that output.
- Changes to Yon settings, profiles or `.env` need a restart.
- A change to `client/manifest.json` reloads Tac. It does not replace the live
  private snapshot.

## Read and bind in every Yon language

These nine alternatives use the same `server/application.json`. Each source
file owns one class and its imports. The empty `yon.<ext>` file marks the
route; its controller owns `GET`. The compiler composes the same-language
behavior-annotated files in `server/helpers/` without making them injectable.

Each endpoint returns only selected non-secret values, never the private
snapshot. `Setting` returns validated data; assigning that data to an instance
is an explicit step.

#### `server/application.json`

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "catalog": {
    "page": {
      "size": 20
    }
  }
}
```

### JavaScript

#### `server/helpers/CatalogSettings.js`

```js
@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  pageSize = 20;
}
```

#### `server/helpers/Endpoint.js`

```js
class Endpoint {
  @Setting("${yon.server.port}")
  port;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  redis;
}
```

#### `server/controllers/ConfigurationController.js`

```js
@RequestRoute("/configuration")
@Public
class ConfigurationController {
  @GetRoute
  getConfiguration() {
    const application = JSON.parse(process.env.YON_APPLICATION_JSON ?? '{}');
    const settings = Behaviors.properties(CatalogSettings, application);
    const endpoint = Behaviors.values(Endpoint, application);
    return { pageSize: settings.pageSize, port: endpoint.port };
  }
}
```

### TypeScript

#### `server/helpers/CatalogSettings.ts`

```ts
@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  pageSize: integer = 20;
}
```

#### `server/helpers/Endpoint.ts`

```ts
class Endpoint {
  @Setting("${yon.server.port}")
  port!: number;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  redis!: string;
}
```

#### `server/controllers/ConfigurationController.ts`

```ts
@RequestRoute("/configuration")
@Public
class ConfigurationController {
  @GetRoute
  getConfiguration() {
    const application = JSON.parse(process.env.YON_APPLICATION_JSON ?? '{}');
    const settings = Behaviors.properties(CatalogSettings, application);
    const endpoint = Behaviors.values(Endpoint, application);
    return { pageSize: settings.pageSize, port: endpoint.port };
  }
}
```

### Python

#### `server/helpers/CatalogSettings.py`

```py
from typing import Annotated
import json
import os

@settings({"prefix":"YON_CATALOG_"})
class CatalogSettings:
    pageSize: Annotated[int, min(1), max(100)] = 20
```

#### `server/helpers/Endpoint.py`

```py
from typing import Annotated
import json
import os

class Endpoint:
    port: Annotated[int, setting("${yon.server.port}")] = 0
    redis: Annotated[str, setting("${yon.redis.url:redis://127.0.0.1:6379}")] = ""
```

#### `server/controllers/ConfigurationController.py`

```py
import json
import os

@request_route("/configuration")
@public
class ConfigurationController:
    @get_route
    def get_configuration(self, request):
        application = json.loads(os.environ.get("YON_APPLICATION_JSON", "{}"))
        settings = Behaviors.properties(CatalogSettings, application)
        endpoint = Behaviors.values(Endpoint, application)
        return {"pageSize": settings["pageSize"], "port": endpoint["port"]}
```

### PHP

#### `server/helpers/CatalogSettings.php`

```php
<?php

#[Settings({"prefix":"YON_CATALOG_"})]
class CatalogSettings {
    #[Min(1)]
    #[Max(100)]
    public int $pageSize = 20;
}
```

#### `server/helpers/Endpoint.php`

```php
<?php

class Endpoint {
    #[Setting('${yon.server.port}')]
    public int $port = 0;
    #[Setting('${yon.redis.url:redis://127.0.0.1:6379}')]
    public string $redis = '';
}
```

#### `server/controllers/ConfigurationController.php`

```php
<?php

#[RequestRoute("/configuration")]
#[Public]
class ConfigurationController {
    #[GetRoute]
    public function getConfiguration($request) {
        $applicationJson = getenv('YON_APPLICATION_JSON') ?: '{}';
        $application = json_decode($applicationJson, true, 512, JSON_THROW_ON_ERROR);
        $settings = Behaviors::properties(CatalogSettings::class, $application);
        $endpoint = Behaviors::values(Endpoint::class, $application);
        return ['pageSize' => $settings['pageSize'], 'port' => $endpoint['port']];
    }
}
```

### Java

#### `server/helpers/CatalogSettings.java`

```java
@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
    @Min(1)
    @Max(100)
    int pageSize = 20;
}
```

#### `server/helpers/Endpoint.java`

```java
class Endpoint {
    @Setting("${yon.server.port}")
    long port;
    @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
    String redis;
}
```

#### `server/controllers/ConfigurationController.java`

```java
@RequestRoute("/configuration")
@Public
class ConfigurationController {
    @GetRoute
    Object getConfiguration() {
        var applicationJson = System.getenv("YON_APPLICATION_JSON");
        var application = (java.util.Map<?, ?>) Json.parse(applicationJson);
        var settings = Behaviors.properties("CatalogSettings", application);
        var endpoint = Behaviors.values("Endpoint", application);
        return java.util.Map.of("pageSize", settings.get("pageSize"), "port", endpoint.get("port"));
    }
}
```

### Kotlin

#### `server/helpers/CatalogSettings.kt`

```kt
@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
    @Min(1)
    @Max(100)
    var pageSize: Int = 20
}
```

#### `server/helpers/Endpoint.kt`

```kt
class Endpoint {
    @Setting("${yon.server.port}")
    var port: Long = 0
    @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
    var redis: String = ""
}
```

#### `server/controllers/ConfigurationController.kt`

```kt
@RequestRoute("/configuration")
@Public
class ConfigurationController {
    @GetRoute
    fun getConfiguration(): Any {
        val applicationJson = System.getenv("YON_APPLICATION_JSON")
        val parsed = Json.parse(applicationJson) as Map<*, *>
        val application = parsed.mapKeys { (key, _) -> key as String }
        val settings = Behaviors.properties("CatalogSettings", application)
        val endpoint = Behaviors.values("Endpoint", application)
        return mapOf("pageSize" to settings["pageSize"], "port" to endpoint["port"])
    }
}
```

### C#

#### `server/helpers/CatalogSettings.cs`

```cs
[Settings({"prefix":"YON_CATALOG_"})]
public sealed class CatalogSettings {
    [Min(1)]
    [Max(100)]
    public int pageSize = 20;
}
```

#### `server/helpers/Endpoint.cs`

```cs
public sealed class Endpoint {
    [Setting("${yon.server.port}")]
    public long port;
    [Setting("${yon.redis.url:redis://127.0.0.1:6379}")]
    public string redis = "";
}
```

#### `server/controllers/ConfigurationController.cs`

```cs
[RequestRoute("/configuration")]
[Public]
class ConfigurationController {
    [GetRoute]
    public object GetConfiguration() {
        var applicationJson = System.Environment.GetEnvironmentVariable("YON_APPLICATION_JSON") ?? "{}";
        var application = BehaviorJSON.Parse(applicationJson);
        var settings = Behaviors.properties(typeof(CatalogSettings), application);
        var endpoint = Behaviors.values(typeof(Endpoint), application);
        var pageSize = settings.Get("pageSize").Integer;
        var port = endpoint.Get("port").Integer;
        return new { pageSize, port };
    }
}
```

### Rust

#### `server/helpers/CatalogSettings.rs`

```rs
#[Settings({"prefix":"YON_CATALOG_","defaults":{"pageSize":20}})]
struct CatalogSettings {
    #[Min(1)]
    #[Max(100)]
    pageSize: i64,
}
```

#### `server/helpers/Endpoint.rs`

```rs
#[allow(dead_code)]
struct Endpoint {
    #[Setting("${yon.server.port}")]
    port: u16,
}
```

#### `server/controllers/ConfigurationController.rs`

```rs
#[RequestRoute("/configuration")]
#[Public]
struct ConfigurationController {}
impl ConfigurationController {
    fn new() -> Self { Self {} }
    #[allow(non_snake_case)]
    #[GetRoute]
    fn get_configuration(&self) -> Json {
        let application_json = std::env::var("YON_APPLICATION_JSON").expect("supervisor configuration");
        let application: BehaviorValue = Json::parse(&application_json).expect("validated configuration").into();
        let settings = Behaviors::properties::<CatalogSettings>(&application).expect("catalog settings");
        let endpoint = Behaviors::values::<Endpoint>(&application).expect("endpoint settings");
        let page_size = settings.get("pageSize").expect("page size").clone();
        let port = endpoint.get("port").expect("port").clone();
        let result = BehaviorValue::object([("pageSize", page_size), ("port", port)]);
        let result_json = result.to_json(1024).expect("bounded output");
        Json::parse(&result_json).expect("valid JSON")
    }
}
```

### Dart

#### `server/helpers/CatalogSettings.dart`

```dart
import 'dart:convert';
import 'dart:io';

@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  int pageSize = 20;
}
```

#### `server/helpers/Endpoint.dart`

```dart
import 'dart:convert';
import 'dart:io';

class Endpoint {
  @Setting("${yon.server.port}")
  int port = 0;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  String redis = '';
}
```

#### `server/controllers/ConfigurationController.dart`

```dart
import 'dart:convert';
import 'dart:io';

@RequestRoute("/configuration")
@Public
class ConfigurationController {
  @GetRoute
  Object? getConfiguration() {
    final applicationJson = Platform.environment['YON_APPLICATION_JSON'] ?? '{}';
    final parsed = jsonDecode(applicationJson) as Map;
    final application = parsed.cast<String, Object?>();
    final settings = Behaviors.properties(CatalogSettings, application) as Map;
    final endpoint = Behaviors.values(Endpoint, application) as Map;
    return {'pageSize': settings['pageSize'], 'port': endpoint['port']};
  }
}
```

### Go

#### `server/helpers/CatalogSettings.go`

```go
// @Settings({"prefix":"YON_CATALOG_","defaults":{"pageSize":20}})
type CatalogSettings struct {
	// @Min(1)
	// @Max(100)
	pageSize int64
}
```

#### `server/helpers/Endpoint.go`

```go
type Endpoint struct {
	// @Setting("${yon.server.port}")
	port uint16
}
```

#### `server/controllers/ConfigurationController.go`

```go
import (
	"encoding/json"
	"os"
)

// @RequestRoute("/configuration")
// @Public
type ConfigurationController struct{}

// @GetRoute
func (controller *ConfigurationController) GetConfiguration() (any, error) {
	var application map[string]any
	err := json.Unmarshal([]byte(os.Getenv("YON_APPLICATION_JSON")), &application)
	if err != nil {
		return nil, err
	}
	settings, err := BehaviorsProperties[CatalogSettings](application)
	if err != nil {
		return nil, err
	}
	endpoint, err := BehaviorsValues[Endpoint](application)
	if err != nil {
		return nil, err
	}
	pageSize := settings.(map[string]any)["pageSize"]
	port := endpoint.(map[string]any)["port"]
	return map[string]any{"pageSize": pageSize, "port": port}, nil
}
```

## A complete worked example

This example composes one Yon unit from four one-class TypeScript files. It
combines `Settings`, `Setting`, `Setup`, `WhenProfile` and
`WhenSetting`. The profile name is itself a property, so the selected
file determines which Setup is active. The handler reads no environment
variable except the reserved snapshot. Select the production file with
`YON_PROFILE=production`.

### `server/application.json`

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "redis": {
    "url": "redis://127.0.0.1:6379"
  },
  "active": {
    "profile": "development"
  },
  "catalog": {
    "page": {
      "size": 20
    },
    "cache": {
      "enabled": false
    }
  }
}
```

### `server/application-production.json`

```json
{
  "server": {
    "port": 9000
  },
  "active": {
    "profile": "production"
  },
  "catalog": {
    "cache": {
      "enabled": true
    }
  }
}
```

### `server/helpers/CatalogProperties.ts`

```typescript
@Settings({"prefix":"YON_CATALOG_"})
class CatalogProperties {
  @Min(1)
  @Max(100)
  pageSize: integer = 20;
  cacheEnabled: boolean = false;
}
```

### `server/helpers/Endpoint.ts`

```typescript
class Endpoint {
  @Setting("${yon.server.port}")
  port!: number;
  @Setting("${yon.redis.url}")
  redis!: string;
  @Setting("${yon.active.profile:development}")
  profile!: string;
}
```

### `server/controllers/ProductsController.ts`

```typescript
@RequestRoute("/products")
@Public
class ProductsController {
  @GetRoute
  async getProducts() {
    const application = JSON.parse(process.env.YON_APPLICATION_JSON ?? '{}');
    const catalog = Behaviors.properties(CatalogProperties, application);
    const endpoint = Object.assign(new Endpoint(), Behaviors.values(Endpoint, application));
    const lifetime = new AbortController();
    const context = await Behaviors.create(ProductionSetup, {
      partition: 'public-catalog', signal: lifetime.signal,
      profiles: [endpoint.profile], properties: application,
    });
    try { return { pageSize: catalog.pageSize, port: endpoint.port }; }
    finally { context.dispose(); lifetime.abort(); }
  }
}
```

### `server/setups/ProductionSetup.ts`

```typescript
@Setup
@WhenProfile('production')
@WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class ProductionSetup {
  static configure() { return {}; }
}
```

The compiler discovers these same-language files within their existing role
folders (ADR 0031). `CatalogProperties` and `Endpoint` carry behavior
annotations only; neither becomes an injectable provider. Unknown
`YON_CATALOG_*` keys fail the `Settings` binding.

What happens:

- With the base file, `Behaviors.create` fails with `inactive-configuration`
  before `configure` runs. The reason: the profile is `development` and the
  cache is off.
- Under `YON_PROFILE=production`, both conditions pass. `endpoint.port` is 9000.
  `catalog.pageSize` is still 20, from the base file.
- The handler reads nothing from the process environment except the reserved
  snapshot. Nothing in this example returns the snapshot itself.

## Deployment authority stays outside the file

Application files cannot select any of these:

- `YON_ISOLATION`
- a Firecracker driver
- a runtime executable
- whether handlers inherit the environment
- native capabilities
- host security policy

These stay explicit deployment decisions. Application configuration gives no
authority.

In Firecracker mode:

- The supervised control program receives the private configuration
  environment value.
- The operator owns the integration. That integration must explicitly copy the
  value into the guest.
- Tachyon does not automatically forward the host environment into a microVM
  (a small virtual machine).
- The existing limits on languages and artifact transfer still apply.

See [ADR 0014](adr/0014-environment-selected-yon-isolation.md) and
[ADR 0023](adr/0023-private-application-configuration.md).

## Migration from v2

1. Move `application.json` and every `application-<profile>.json` from the
   project root into `server/`.
2. Move a root `manifest.json` into `client/`.
   If a file stays at the root, the build stops and says where the file
   belongs.
3. Move the members of the `yon` section to the top level of
   `server/application.json`, and remove the `yon` key. Do this in each profile
   too.
4. Move the `tac` section of `application.json` into `client/manifest.json` as
   its `tac` member. If the project has no manifest, create one. `name` and
   `start_url` are enough to start.
5. `.env` stays in the project root.
6. Change any client that loaded `/.tachyon/application.json` to load
   `/manifest.webmanifest`.

`ty migrate` reports each of these moves.

These do not change: paths such as `yon.server.port`, `TAC_`/`YON_` names,
`Setting` expressions and the `YON_APPLICATION_JSON` snapshot.

See [ADR 0034](adr/0034-tac-configuration-is-the-web-app-manifest.md) and the
[v3 schema](../api/application-config/v3/schema.json).

## Migration from v1

1. Move server/datasource/redis into the yon namespace.
2. Convert properties.TAC_*/YON_* to nested paths.
3. Move secrets into .env.
4. Remove JSON placeholders.
5. Update Setting paths.
6. Pass the canonical snapshot to explicit property binding.

See [ADR 0026](adr/0026-json-results-and-separated-configuration.md) and the
[v2 schema](../api/application-config/v2/schema.json). The v1 schema stays as a
historical record. Tachyon does not silently accept the old way of writing
configuration.
