# ADR 0052: The Manifest Is the Tac Configuration

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0029](0029-pages-compose-components.md) (where site-wide
  `scripts` and `styles` are declared) and
  [ADR 0034](0034-tac-configuration-is-the-web-app-manifest.md) (the `tac`
  member now also holds Tachyon's own settings)
- Supersedes: [ADR 0040](0040-tac-config-is-a-class.md) (`tac.config` is
  removed) and the `tachyon.json` file of the Phase 4 native slice
- Preserves: the web app manifest as a public document, its location at
  `client/manifest.json`, `TAC_` names and `tac.` paths, and every limit and
  check the old fields had

## Context

A Tac project kept its configuration in three places:

- `client/manifest.json`, the web app manifest, for the name, entry route,
  icons, window and public Tac values;
- `tac.config.js` (or `.mjs`, `.ts`), a class whose static fields held the
  native application identity, site-wide scripts and styles, service-worker
  cache rules and page metadata, plus a `postBundle` hook;
- `tachyon.json`, an older file with the native application identity.

The same value, such as the application name, could be written in two files,
and an order decided which one won. `tac.config.js` was code: every build
started Node.js or Bun to read it, and the website used `postBundle` only to
write `sitemap.xml` and `robots.txt`.

## Decision

`client/manifest.json` is the only Tac configuration file, apart from
`jsconfig.json` and `tsconfig.json`.

- The standard manifest members keep their meaning and feed native builds:
  `name` (or `short_name`) names the app, `start_url` is the entry route, and
  `icons` gives the app icons.
- The `tac` member holds Tachyon's own settings next to the public values that
  services bind:
  - `application`: `id` (the bundle id), `version` and `window` (`width`,
    `height`, `minWidth`, `minHeight`, `controls`). Unknown keys are refused.
    Without them, the id comes from the folder name, the version is 1.0.0 and
    the window is 1024 by 768.
  - `scripts` and `styles`: up to 8 local paths each, linked into every page.
  - `styleSources`: optional declarations keyed by exact local `/shared/` stylesheet
    URL. `{ "kind": "foundation" }` receives the narrow colors/fonts/reset audit;
    an undeclared legacy `styles` string receives the same audit. A pinned vendor
    declares `{ "kind": "vendor", "package", "version", "license", "sha256" }`
    and the build verifies captured source bytes. CSS imports retain file/byte/
    depth/path/encoding/no-symlink bounds and cannot hide undeclared application
    layout. The build error names the limit or rule that a stylesheet breaks.
  - `scriptSources`: optional exact local shared vendor-script declarations with
    `package`, `version`, `license` and `sha256`. Only byte-verified declared vendors
    receive the authored-script exception; folder and filename do not grant it.
  - `cache`: up to 64 service-worker read rules, `{ "path", "policy" }`.
  - `metadata`: `defaults` and `routes` (keyed by route, such as
    `/docs/_topic`) with `title`, `description`, `canonical`, `image`,
    `siteName`, `lang` and `noindex`. A page gets its route entry over the
    defaults. Page metadata is data in the manifest, not markup in the page,
    because a page holds only components (ADR 0029).
  - `origin`: the site's public address. A `canonical` or `image` that
    starts with `/` is joined to it.
  - `sitemap`: for each page route with one path variable, a data file under
    `/shared/`, a JSON pointer to a list in it, and optionally the member of
    each listed object that fills the variable.
- These Tachyon members have fixed shapes and are read by the build. The
  service-configuration rules of ADR 0034 (lowercase keys, key and size
  bounds) apply to the other members of `tac`.
- When `tac.origin` is set, the build writes `sitemap.xml` and `robots.txt`.
  The sitemap lists every page route without a path variable, except pages
  whose metadata sets `noindex`, and every value `tac.sitemap` lists for a
  dynamic page. `robots.txt` allows everything and names the sitemap.
- The `postBundle` hook is removed. The build no longer starts a JavaScript
  runtime to read configuration.
- A `tac.config.js`, `tac.config.mjs`, `tac.config.ts` or `tachyon.json` at
  the project root stops discovery, builds and server start. The message says
  where each value moved. The manifest member `tachyon` (the earlier home of
  `id`, `version` and `window`) is refused the same way. `ty migrate fix
  --write` moves plain values into the manifest and lists the rest as manual
  steps.

## Consequences

- One file answers "what is this application called, where does it start,
  and how is it published". There is no precedence order to learn.
- Everything in the manifest ships to every browser, as ADR 0034 says. The
  application id, version, window, metadata and sitemap settings are public
  by design. A secret must never be written there.
- Computed configuration, such as an origin read from an environment
  variable or a sitemap built by a script, is no longer possible inside the
  build. A project that needs it generates the manifest or the files before
  it runs `ty bundle`.
- Builds are faster and need no JavaScript runtime for configuration.

## Amendment (2026-10-10): Blocking scripts run before the first paint

Every `tac.scripts` entry is a deferred `type="module"` script after the Tac
runtime, `/.tachyon/tac-client.js`. A module script runs only after the
document is parsed, and a browser may paint before that. A page that must
change the document before the first paint, such as the website applying a
visitor's saved light theme, could show one frame of the default theme
first. The site's Content Security Policy (`script-src 'self'`) refuses
inline scripts, so the work cannot move into the page head as text.

### Decision

- `tac.blockingScripts` lists up to 8 local paths, with the same path rules
  and limit as `tac.scripts`.
- The build links each one into every page, in the declared order, as
  `<script src="..." data-tachyon-runtime></script>`: a classic script
  without `type`, `defer` or `async`, placed directly before the Tac
  runtime's module tag. The parser stops until a classic script has loaded
  and run, so it runs before the body is parsed and before the first paint.
  `script-src 'self'` allows it without a hash or nonce.
- `blocking="render"` on a module tag was not chosen. Of the pinned
  Playwright engines (Chromium 151, Firefox 153 and WebKit 26.5), Firefox
  does not expose `HTMLScriptElement.prototype.blocking`, so it ignores the
  attribute and the module stays deferred. A classic script blocks
  rendering in all three.
- `tac.scripts` does not change: each entry is still a deferred module
  after the runtime. A vendor in `tac.scriptSources` must still appear in
  `tac.scripts` (`TY1365`). A blocking script is an authored shared script
  and receives the authored-script check (`TY1364`).

### Consequences

- The first paint waits for every blocking script to load and run. Keep the
  list short and each file small.
- A blocking script is a classic script: it cannot use `import` or `export`,
  and its top-level class becomes a global name of the page.

## Amendment (2026-10-11): The theme color reaches every page head

A browser reads the manifest's standard `theme_color` only for an installed
application. A page in a browser tab, and a native build's page, colored the
browser's interface only with `<meta name="theme-color">` in the page head,
and an application could not add that tag: the build writes the head.

### Decision

- When `theme_color` in `client/manifest.json` is a text value that is not
  blank, the build adds `<meta name="theme-color" content="...">` to every
  page head, after the manifest and icon links. The value is trimmed and
  HTML-escaped, so it stays one attribute value.
- A missing, blank, number or other non-text `theme_color` writes no tag and
  still does not stop the build. The manifest is still published as written.
- The manifest models one theme color. A light and dark pair
  (`media="(prefers-color-scheme: dark)"`) is not added until the manifest
  has a member for it.

### Consequences

- The color is declared once, in the manifest, for installation, browser
  tabs and native pages.

## Amendment (2026-10-18): Native hosts do not read the theme color yet

The 2026-10-11 amendment writes `<meta name="theme-color">` into every page
head, native builds included. No native host reads that tag or the manifest's
`theme_color` today: the macOS, Windows and Linux windows and the iOS and
Android status bars keep their system colors. A native build that declares
`theme_color` now prints a note that says so (ADR 0102). The plan items WIN1,
WIN2 and WIN5 apply the color on Windows, Linux and mobile.

## Amendment (2026-10-18): Versions and build numbers reach every target

`tac.application.version` reached only part of each native app. Windows wrote
`1.0.0.0` in the executable's manifest and had no version resource, so
Explorer and installers saw no version. Android `versionCode` and Apple
`CFBundleVersion` were always 1, so a store or a device could not tell two
builds of one version apart. Linux wrote the version only as
`X-Tachyon-Version`.

### Decision

- `tac.application.build` is the build number: a whole number from 1, and 1
  when absent. Zero, a negative number, a fraction or text stops the build
  (`TY1601`).
- Each target takes the version and the build from the manifest:
  - macOS and iOS: `CFBundleShortVersionString` is the version and
    `CFBundleVersion` is the build.
  - Android: `versionName` is the version and `versionCode` is the build.
  - Windows: a `VERSIONINFO` resource in the executable and the
    `assemblyIdentity` of its manifest. The four-number file version is the
    version's numbers, padded with zeros to three, then the build, so `1.4`
    with build 12 is `1.4.0.12`. `ProductVersion` is the version as written.
  - Linux: the desktop entry format has no application version key (its
    `Version` key is the format's own version). The desktop entry keeps
    `X-Tachyon-Version` and adds `X-Tachyon-Build`. Package metadata, such as
    AppStream, comes with Linux packaging.
- A build refuses a version a target cannot carry, and only for that target
  (`TY1601`, naming the target):
  - macOS and iOS need one to three numbers separated by dots, as the App
    Store does, so `1.4.0-beta` is refused there.
  - Windows needs the same, with each number at most 65535, and a build of
    at most 65535.
  - Android needs a build of at most 2100000000, the largest `versionCode`
    Google Play accepts.
  - The version stays free text for Android's `versionName` and for Linux.
- The check runs before any native tool, so a staged build without packaging
  refuses the same values.

### Consequences

- Raise `tac.application.build` for every build you hand out, and keep the
  version numeric when the app ships to macOS, iOS or Windows.
- Every Windows build now compiles a resource script with `rc.exe`, even
  without an icon. `rc.exe` comes with MSVC's developer environment, which
  the Windows host already needs.
- The Windows manifest still sits at the bundle root, where Windows does not
  read it. Its version is correct, but the executable's version resource is
  what Windows shows.

## Amendment (2026-10-18): The Windows window follows the manifest

Only the macOS host read `tac.application.window`. The Windows host always
opened a 1024 by 768 window, had no minimum size, and did not apply the
standard `theme_color` and `display` members; the amendment "Native hosts do not
read the theme color yet" made the build say so in a note. This amendment applies them on Windows (plan item
WIN1).

### Decision

- The Windows host opens the window's client area at `width` by `height`
  device-independent pixels, scaled by the window's DPI, and answers
  `WM_GETMINMAXINFO` with `minWidth` and `minHeight` scaled the same way. It
  keeps the system minimum when that is larger, and sets no minimum for an
  edge that the manifest leaves out.
- A `theme_color` written as `#rgb` or `#rrggbb` becomes the caption color
  (`DWMWA_CAPTION_COLOR`), with black or white caption text, whichever has
  more contrast. Windows 10 does not support the attribute and keeps the
  system color; named colors and other CSS forms are not applied natively.
- `display` must be one of the web manifest values `fullscreen`,
  `standalone`, `minimal-ui` or `browser`; any other value stops every build
  with `TY1601`. `fullscreen` starts the Windows app borderless and covering
  its monitor, with the same code as the `fullscreen` window control. The
  other values open a standard window.

### Consequences

- A manifest's size, minimum size, theme color and display mode give the
  same first impression on Windows as on macOS and in an installed web app.
- A fullscreen start can be left only through the `fullscreen` window
  control or by closing the app, as in a fullscreen web app.
- The host links `dwmapi.lib`.
- Evidence: unit tests of the generated host, a C test that runs the frame
  code against Win32 stand-ins, and a MinGW compile against the Win32
  headers. The real-window check
  (`the_windows_window_frame_drives_a_real_win32_window`) passed with MSVC
  on Windows 11 ARM64 over SSH on 2026-10-19 (RW1), for both the sized and
  the fullscreen start. The caption color still needs a look on a signed-in
  desktop.

## Amendment (2026-10-18): The Linux window follows the manifest

The Linux host still opened every app at 1024 by 768 with no minimum size,
and did not apply `theme_color` or `display`. This amendment applies them on
Linux (plan item WIN2), with the same color and display rules as the Windows
amendment above.

### Decision

- `width` and `height` are the size of the page inside the window, as on
  macOS and Windows. GTK sizes the whole window, so on the first frame the
  host measures what the title bar or header bar takes and grows the window
  by that much. A granted `resize` window control also sets the page size.
- `minWidth` and `minHeight` are the page's size request, so the window
  cannot shrink the page below them. An edge the manifest leaves out has no
  minimum.
- A `theme_color` written as `#rgb` or `#rrggbb` puts a GTK header bar on the
  window, with that background and black or white title text, whichever has
  more contrast. The build writes the color as `#rrggbb`, never as the
  authored text. The header bar replaces the window manager's title bar.
  Other color forms keep the standard title bar, and the build's note says
  so.
- `display: "fullscreen"` starts the window full screen. The other values
  open a standard window, and the build's note says so.

### Consequences

- The same manifest gives the same page size on macOS, Windows and Linux.
- An app with a hex `theme_color` always draws its own title bar on Linux,
  also under desktops that otherwise draw title bars on the server side.
- Evidence: `linux_window_appearance_tests` compiled the generated host with
  GTK 4.14.5 and WebKitGTK 2.52.6 and ran it under Xvfb with a window
  manager in an x86_64 cloud container. On 2026-10-19 both of its tests
  also passed on WSL (Ubuntu 24.04.5, aarch64, Xvfb with xfwm4) on develop
  `38eaed29`, with the other Linux host tests (`docs/SUPPORT_TIERS.md`).
  Still owed: a physical Linux desktop and Wayland.

## Amendment (2026-10-19): The Linux page size holds under Wayland

Under a Wayland compositor the declared page size was lost: the window
opened at the declared size, the host grew it on the first frame by the
height of the header bar, and the compositor's first configure then set the
window back, so a 900x640 page showed at 900x566. On X11 the window manager
does not resize a client, so the X11 runs passed.

### Decision

- The host still measures the title bar or header bar on the first frame.
  It then asks for the grown size again on later frames until the page has
  the declared size, for at most 120 frames, and stops at once when the
  window is maximized or full screen.

### Consequences

- Evidence (RL1, 2026-10-19): `linux_window_appearance_tests` passed in a
  headless mutter 46.2 Wayland session (`scripts/linux/compositor-test.sh`)
  and under Xvfb with xfwm4, in an x86_64 Ubuntu 24.04 cloud container
  (GTK 4.14.5, WebKitGTK 2.52.6). On WSL Ubuntu 24.04 (aarch64) the same
  tests passed under Xvfb with xfwm4. `docs/SUPPORT_TIERS.md` has the runs.
- A Wayland app cannot see that it is minimized (xdg-shell has no such
  state), so the Wayland run checks that a granted `minimize` call succeeds,
  not that the window went away.
