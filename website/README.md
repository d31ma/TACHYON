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
| `bun run content` | Writes `client/shared/data/docs.json`, `features.json` and `languages.json` from `content/`. |
| `bun run content:check` | Fails when those files do not match `content/`. |
| `bun run test` | Runs the site tests. |
| `ty test .` | Runs the services' test units. |
| `bun run verify:snippets` | Builds and runs every code example on the site. |
| `bun run verify:snippets --suite=tsc` | Type-checks every JavaScript and TypeScript example project with `tsc`, the way a `ty init` project checks itself. |
| `bun run verify:snippets --feature=<id>` | Checks one feature, such as `--feature=iteration`. It runs only the features suite. |
| `bun run verify:snippets --section="<heading>"` | Checks one docs section, such as `--section="Security"`. It runs only the guides suite. Add `--suite=` to choose another suite, for example `--suite=tsc`. |
| `bun run verify:snippets --no-cache` | Builds every example again, even one that passed before with the same inputs. Use it for a scheduled full run. |
| `bun run coverage:languages` | Lists, for each docs section and feature, which Tac and Yon languages have a tab and why each other language has none: a missing example, a build error that refuses it (with its `TY` code), or not applicable. A section or feature marks a language as not applicable with `"notApplicable": ["js", "ts"]`, and it counts only when its text names the language and says why. Add `--json` for a report that tools read, or `--strict` to fail on a gap that has no example and no stated reason. |
| `bun run styles` | Refreshes each component's generated Tailwind region. Run it after you change a template. |
| `bun run styles:check` | Checks generated component regions and shared variable registrations without writing files. |
| `bun run check:responsive` | Checks the layout at many screen widths. |
| `node scripts/check-migration.mjs` | Runs an extra browser check on a website build you made before with `ty bundle website --out-dir <dir>`. Set these environment variables first: `TAC_BIN` (the `ty` that made the build), `TAC_WEBSITE_OUTPUT` (the build folder), `TAC_WEBSITE_SOURCE_COMMIT` (the commit you built), `TAC_WEBSITE_ACCEPTANCE_RECEIPT` (where to write the JSON result) and `TAC_BROWSER_DEPENDENCIES` (a `node_modules` folder with Playwright). |

`verify:snippets` runs before the site is published. An example that does not
build is worse than no example.

`verify:snippets` remembers each build, test-unit and `tsc` step that passed.
The next run skips a step when all of its inputs are the same: the example's
files, the `ty` binary, the lock files, the verifier scripts, the `TAC_*`
settings and the version of every toolchain (`rustc`, `dotnet`, `swiftc`,
`go`, `tinygo`, `emcc`, `dart`, `kotlinc`, TeaVM and the others). A skipped
step shows `(cached)`. HTTP and browser checks always run. A step that failed
is never remembered. The cache lives in `.step-cache` inside the staging
folder (`TAC_SNIPPETS_SCRATCH`, or `tachyon-snippets` in the system temporary
folder), never in the repository.

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
`TACHYON_WEBSITE_DEPENDENCIES` to the website's `node_modules` folder when you
run the generator. You do not need a dependency link inside the synced source.

The theme is dark until the visitor picks another one. `site.css` gives the
page a dark background while the root has no `w-theme` attribute, so the first
paint is dark. `shared/scripts/ThemeDefault.js` runs before DuVay's behaviour
script and saves `dark` when nothing is saved, because that script falls back
to `light`. DuVay's script then applies the saved theme, and its
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
