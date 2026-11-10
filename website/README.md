# Tachyon website

This is Tachyon's own website, and it is built with Tachyon. Tac views run in
the browser, and services give them their behavior (ADR 0030). The site is a
static bundle, so search also runs in the browser.

## Folders

```text
client/
  pages/        the routes: /, /docs, /docs/:topic, /docs/features/:id
  components/   the site shell, the home page sections and the docs parts (tac.html, and tac.css if needed)
  services/     one @Service class per file, in JavaScript, TypeScript, Dart, Rust, C#, F#, Go, Ruby, Kotlin or Swift
  utils/        utility classes with only static methods, such as NavigationUtils
  models/       TypeScript interfaces and Go structs the services share (ADR 0041, ADR 0045)
  tests/        a test unit for every service, plus separate test-only setups
  shared/       styles, the DuVay components, scripts, and the site data in data/
content/        every code example as a real file, with the docs text (see "Code examples")
runs/           one sample recorded run of a Yon example (see "Recorded runs")
server/         the Yon routes that run examples live: POST /api/snippets/{id}/runs (see "Live runs")
runner/         the snippet host and the runner image (Bun; not part of the site bundle)
client/manifest.json   the Tac configuration: page metadata, the shared script and stylesheet,
                       the sitemap settings and the native identity (ADR 0052)
```

A view reads a service by its instance name: `{search.results}` reads the
`results` field of `SearchService`.

Each annotated source file holds one class and its imports (ADR 0044). Rust
keeps the struct and that struct's `impl` blocks together, and Go keeps the
type and the methods on it together. Plain helper code becomes
a private method of its owner or a static method in `utils/`; pure data types belong in `models/`.
A `Test` class selects a sibling `<Name>TestSetup.<ext>` file by
its `configuration` name. The test setup never enters the site bundle.
Published examples follow the same rule in every supported language.
Behavior-only annotated helpers can also occupy separate files in existing
role folders without a stereotype; composition does not make them injectable.
Included role files retain their existing import restrictions: no relocatable
relative dependency imports. Empty controller-owned Yon markers contain no imports.

## Commands

| Command | What it does |
|---|---|
| `bun install` | Installs the Tailwind tools. |
| `bun run dev` | Writes the site data from `content/`, then starts the development server. |
| `bun run bundle` | Writes the site data from `content/`, then builds the site into `dist/web`. |
| `bun run content` | Writes `client/shared/data/docs.json`, `guides.json`, `features.json` and `languages.json` from `content/`. `content/generated-files.json` lists them. |
| `bun run content:check` | Fails when those files do not match `content/`. |
| `bun run test` | Runs the site tests. |
| `ty test .` | Runs the services' test units. |
| `bun run verify:snippets` | Builds and runs every code example on the site. |
| `bun run verify:snippets --suite=tsc` | Type-checks every JavaScript and TypeScript example project with `bun check`, using the `jsconfig.json` files that `ty init` writes. The suite keeps the name `tsc`; `bun check` reports the same diagnostics. |
| `bun run verify:snippets --feature=<id>` | Checks one feature, such as `--feature=iteration`. It runs only the features suite. |
| `bun run verify:snippets --section="<heading>"` | Checks one docs section, such as `--section="Security"`. It runs only the guides suite. Add `--suite=` to choose another suite, for example `--suite=tsc`. |
| `bun run verify:snippets --shard=<index>/<count>` | Runs one share of the steps, such as `--shard=2/3`, so several machines can split a run. Shard 1 also runs the checks that are not split. The shards of one count together run every step once. |
| `bun run verify:snippets --no-cache` | Builds every example again, even one that passed before with the same inputs. Use it for a scheduled full run. |
| `bun run coverage:languages` | Lists, for each docs section and feature, which Tac and Yon languages have a tab and why each other language has none: a missing example, a build error that refuses it (with its `TY` code), or not applicable. A section or feature marks a language as not applicable with `"notApplicable": ["js", "ts"]`, and it counts only when its text names the language and says why. Add `--json` for a report that tools read, or `--strict` to fail on a gap that has no example and no stated reason. |
| `bun run styles` | Refreshes each component's generated Tailwind region. Run it after you change a template. |
| `bun run styles:check` | Checks generated component regions and shared variable registrations without writing files. |
| `bun run check:responsive` | Checks the layout at many screen widths. |
| `node scripts/check-migration.mjs` | Runs an extra browser check on a website build you made before with `ty bundle website --out-dir <dir>`. Set these environment variables first: `TAC_BIN` (the `ty` that made the build), `TAC_WEBSITE_OUTPUT` (the build folder), `TAC_WEBSITE_SOURCE_COMMIT` (the commit you built), `TAC_WEBSITE_ACCEPTANCE_RECEIPT` (where to write the JSON result) and `TAC_BROWSER_DEPENDENCIES` (a `node_modules` folder with Playwright). |

`verify:snippets` runs before the site is published. An example that does not
build is worse than no example.

`verify:snippets` remembers each build, test-unit and type-check step that
passed. The next run skips a step when all of its inputs are the same: the
example's files, the `ty` binary, the lock files, the verifier scripts, the
`TAC_*` settings and the version of every toolchain (`rustc`, `dotnet`,
`swiftc`, `go`, `tinygo`, `emcc`, `dart`, `kotlinc`, TeaVM and the others).
A skipped step shows `(cached)`. HTTP and browser checks always run. A step that failed
is never remembered. The cache lives in `.step-cache` inside the staging
folder (`TAC_SNIPPETS_SCRATCH`, or `tachyon-snippets` in the system temporary
folder), never in the repository.
Nothing prunes that folder, and a full run adds gigabytes to it. Delete the
folder after a large run.

The examples use the same Tachyon cache as `ty` itself: `TAC_CACHE_DIR`, or
the platform's default cache folder that `ty doctor` shows. A runtime that a
setup step installed there, such as Pyodide, is the one the examples use.

Tac Rust, Swift, Kotlin and C# examples are compiled for the web, the way the
site ships them. When a language's web toolchain is missing, the verifier
stages native source instead and says so; with
`TAC_SNIPPETS_REQUIRE_TOOLCHAINS=1` the missing toolchain is a failure. Each
example keeps its `dist` folder between runs, so `ty bundle` reuses the
compressed files of a runtime that did not change.

## Code examples

Every example on the site is a real file under `content/`. Each page, topic,
section, feature and language entry has a small JSON file with its text. Its
`files` list gives the tabs in display order: `name` is the label the site
shows, and `code` names the file:
`{ "file": "server/controllers/ItemsController.py" }`.
`{ "include": "streaming/section.json" }` reads another JSON file. Paths are
relative to the JSON file and stay inside `content/`.

A docs section has a `heading` and may have a `summary` and a `body`. The site
shows the `summary` as a lede paragraph under the heading, then the `body`.
Search, the reading time and the language coverage audit read both.

A section folder is laid out like a small Tachyon project: each example sits
at the path its tab shows. Three names differ so that git treats them as
examples: `example.env`, `example.gitignore` and `example.tachyonrc` show as
`.env`, `.gitignore` and `.tachyonrc`. A section with one example uses
`example.<ext>`. A `.txt` file is text that is not code: a tree, a table or
terminal output.

To change an example, edit its file, then run `bun run content`. To add one,
create the file in the section's folder at the path its tab shows, and add an
entry to `files`. Do not edit `client/shared/data/docs.json`, `features.json`
or `languages.json`: they are generated, and `bun run test` fails when they do
not match `content/` or when a file in `content/` is not used.
`bun run verify:snippets` builds and runs the examples. The "Edit this page"
link on a docs page opens the JSON file of that topic or feature.

`content/` is not part of the site: `ty bundle`, `ty preview` and `ty test`
read only `client/`, `server/` and the root configuration, and the root type
check skips it.

### Recorded runs

A Yon controller example with an HTTP route has a **Run** button. The button
shows the requests that `verify:snippets` sent to the controller and the
answers it gave: method, path, body, status, content type and response body.
The site does not run code: it shows what the example did when the release
was verified. A socket controller has no Run button.

Every controller tab in the generated data has `"run": "<id>"`, where `<id>`
is the first 16 hex digits of the SHA-256 hash of the controller's path under
`content/`. The id does not depend on any recording, so the generated data in
git stays the same. The browser loads `/shared/data/runs/<id>.json` when the
visitor presses Run; when that file is missing, the button says "No recorded
output in this build". On `tachyon.del.ma` and `staging.tachyon.del.ma` the
file comes from the API origin (see Live runs), because Tac sends every HTTP
client call there once the page has one; the runner container serves the
same site build, recordings included. So when the API is down, the recorded
output does not show either. A local preview loads it from its own origin.

The recordings are refreshed and committed at each release. They live in
`runs/`, at each controller's path under `content/`:

1. `TAC_SNIPPETS_RECORD=1 bun run verify:snippets` records each controller
   that passes to `.runs/` in the staging folder (`TAC_SNIPPETS_SCRATCH`, or
   `tachyon-snippets` in the system temporary folder), at its path under
   `content/`. Each recording keeps a SHA-256 hash of every example file the
   run used. The release gates run this on the release commit, with every
   server toolchain, Redis and a `ty` built with all storage features.
2. Before the release branch is cut, the recordings replace the contents of
   `runs/` in a commit to `develop`
   (`rsync -a --delete <staging folder>/.runs/ runs/`).
3. `bun run content` publishes each recording in `runs/` whose files still
   match their hashes to `client/shared/data/runs/<id>.json`, and lists the
   recordings it left out. An example that changed after it was recorded
   shows "No recorded output" until the next release records it again.
4. `bun run bundle`, which the release workflow runs, runs step 3 and then
   `ty bundle`, which copies them into the site.

`client/shared/data/runs/` is not in git. `bun run content --runs <folder>`
publishes the recordings in another folder instead of `runs/`, for example
fresh ones from a local `verify:snippets`.

### Live runs

Under the recorded output, the Run panel has a form, filled in from the
recording, where a visitor can change the path (with a query after `?`) and
the body, then press **Run live**. The page sends the run to
`POST /api/snippets/{id}/runs` on the API origin. The script
`client/shared/scripts/YonOrigin.js`, the first of `tac.scripts`, so it runs
after the page's meta tags, chooses it from the page's host name, so
one build works on both sites: `tachyon.del.ma` calls
`https://api.tachyon.del.ma`, `staging.tachyon.del.ma` calls
`https://api.staging.tachyon.del.ma`, and any other host, such as a local
preview, calls its own origin. `tac.yon.origin` in `client/manifest.json`
makes the build write the `tachyon-yon-origin` meta tag with the production
API origin; the script changes the tag for the staging host and removes it on
any other host. Tac sends the request to that origin over CORS without
cookies. `tac.outgoing.hosts` adds both API origins to the page's
`connect-src`. The proxy in front of the API answers
CORS for the site origin, for the runs and for `GET /shared/data/runs/*`,
so `server.origins` stays empty. When nothing answers, as on a static preview, the page says so and keeps the
recorded output.

A live run goes through three parts, all in one container:

1. `ty start website` serves `server/`. `SnippetRunController` is `@Public`.
   It passes the run, and the client address that `ty` puts in `X-Real-IP`
   (`server.proxies` trusts only loopback), to the snippet host. The address
   reaches the host as `X-Snippet-Client`. `server/application.json` lists
   the host as `snippets.localhost` in `outgoing.hosts`; that is the only
   loopback name a confined handler may reach. `server.hosts` lists
   `api.tachyon.del.ma` and `api.staging.tachyon.del.ma`.
2. The snippet host (`runner/`, run with `bun runner/main.ts`) listens on
   `127.0.0.1:8100` and refuses other peers. It refuses an example that was
   not staged for this build, a path that is not one of the example's routes,
   a body over 16 KiB or more than 16 query values. It allows 10 runs a
   minute and 200 a day for each client address (429 with `Retry-After`),
   runs at most 4 at once with 16 waiting (503 beyond that), and stops an
   answer at 64 KiB and a run at 10 seconds (`outcome: "timeout"`).
3. For each example, the host starts the example's own `ty start` on first
   use, from a copy in a temporary folder, with a cleared environment and
   `YON_ISOLATION=process`, so every request gets a fresh process under the
   operating system's egress confinement. An example server that has been
   idle for 60 seconds stops and its folder is deleted; at most 4 run at once.

`TAC_SNIPPETS_STAGE_RUNNABLE=<folder> bun run verify:snippets` writes each
controller that passes to `<folder>/<id>/project`, with `runnable.json`
(its source, language and route templates). Examples that need FYLO are
left out. `node scripts/stage-runner-image.mjs --out <context> --ty <Linux ty>
--runnable <folder> --website-dist <dist/web>` gathers the image context, and
`docker buildx build --build-arg TOOLCHAIN_IMAGE=<Linux toolchain image>
<context>` builds `runner/Dockerfile`. The image keeps only what the Yon
handler languages need at run time, on a plain Ubuntu base of the same
release as the toolchain image:

- From the toolchain image it copies `/opt/tachyon` without Swift (Yon has no
  Swift handlers), .NET's WebAssembly workloads and older runtimes, Rust's
  clippy, rustfmt and WebAssembly target, nextest and sccache. Every
  compiler stays: `ty start` checks each one and puts its version in the
  handler cache key, so a bundled handler still needs the compiler that
  built it.
- From Ubuntu it installs the JDK, `php-cgi` and Lua packages that the
  toolchain image has, so their versions stay in step with
  `vm-setup-linux.sh`, plus clang, g++, `perl` (the relay examples) and the
  libraries Ruby and .NET load.
- It adds the AWS Lambda Web Adapter, `ty`, the website and the staged
  examples. The build bundles every example and then deletes Tachyon's
  shared copy of each compiled handler, since each example keeps its own.

The container starts `bun runner/main.ts`, which starts the snippet host and
`ty start website` on port 8000.

To try it on this machine, stage some examples, run `ty bundle .` in each
`project` folder, build the site, and run
`SNIPPET_HOST_PROJECTS=<folder> SNIPPET_HOST_TY=<ty> SNIPPET_HOST_WEBSITE=$PWD
WEBSITE_PORT=8040 bun runner/main.ts`; the site and its API are then at
`http://localhost:8040`.

## Styles

DuVay gives the fixed structure, and Tailwind gives the small details. Both use
the same class specificity, so the stylesheet orders them with layers:

```css
@layer duvay, theme, base, components, brand, utilities;
```

`tailwind.src.css` turns DuVay's design tokens into Tailwind utilities. So a
template writes `text-sm` or `border-line` instead of an inline style, and both
follow the light and dark themes. Tailwind is generated without Preflight, its
base reset, because DuVay owns the base layer.

`client/shared/styles/site.css` holds only the theme variables, the default
font and color of the body, and the contrast and reduced-motion resets. It
imports the DuVay stylesheet, and the manifest's `styleSources` entry pins that
file. The two DuVay scripts are listed in `tac.scripts` like any other script,
and `scriptSources` gives the hash of each one. The vendor folder records the
DuVay version, its MIT license and where each file came from.

`duvay-wc-site.min.js` holds only the DuVay components that the views use
(`w-app-bar`, `w-btn`, `w-container`, `w-divider`, `w-expansion-panel` and
`w-expansion-panels`). The full component bundle is about 134 KiB as sent and
put every route over the script budget. `PROVENANCE.json` names the DuVay
commit, the source files and their hashes, and the bundler. When a view starts
to use another DuVay component, add its module to `build.modules` and
`build.inputs`, then run `DUVAY_SOURCE=<DuVay checkout> bun run duvay`. It checks the source
hashes, rebuilds the file and stops with the new hash when the output changed;
put that hash in `PROVENANCE.json` and `tac.scriptSources`.

Each component keeps its layout and text styles in `tac.css`, next to its HTML.
`bun run styles` reads each template on its own with the pinned Tailwind
scanner and compiler. In that component's `tac.css` it replaces only the
generated region, and it keeps the rules you wrote outside that region.
Tailwind's CSS `@property` registrations are global, so they live in
`tailwind-properties.css`; Tachyon still scopes the generated selectors to
their component. The site loads no shared utility stylesheet and no script that
makes styles.

If this machine keeps dependencies outside the source folder, set
`TAC_WEBSITE_DEPENDENCIES` to the website's `node_modules` folder when you
run the generator. You do not need a dependency link inside the synced source.

The theme is dark until the visitor picks another one. `site.css` gives the
page a dark background while the root has no `w-theme` attribute, so the first
paint is dark. `shared/scripts/ThemeDefault.js` is listed in
`tac.blockingScripts`, so the build links it as a classic script before the Tac
runtime and it runs before the first paint. It saves `dark` when nothing is
saved, because DuVay's behaviour script falls back to `light`, and it sets the
saved theme on the root `w-theme` attribute, so a saved light theme is light
from the first paint. DuVay's script then keeps that theme, and its
`w-theme-toggle` button changes and saves it.
Preference access stays in services. `DocumentationUtils` and `PlatformUtils`
contain only stateless static methods at the canonical shared utility root.

## Sitemap and robots

The build writes `sitemap.xml` and `robots.txt` itself, because
`client/manifest.json` sets `tac.origin`. The sitemap lists the static pages
and, through `tac.sitemap`, one address for each guide in `docs.json` (`order`)
and each feature in `features.json` (`features[].id`), so it matches the site
that was built. Every page has a canonical URL and Open Graph and Twitter
metadata from `tac.metadata`.
