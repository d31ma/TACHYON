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
| `bun run coverage:languages` | Lists, for each docs section and feature, which Tac and Yon languages have a tab and why each other language has none: a missing example, a build error that refuses it (with its `TY` code), or not applicable. Add `--json` for a report that tools read, or `--strict` to fail on a gap that has no example and no stated reason. |
| `bun run styles` | Creates `tailwind.css` again. Run it after you change a template. |
| `bun run styles:check` | Fails when the committed `tailwind.css` is out of date. |
| `bun run check:responsive` | Checks the layout at many screen widths. |

`verify:snippets` runs before the site is published. An example that does not
build is worse than no example.

## Code examples

Every example on the site is a real file under `content/`. Each page, topic,
section, feature and language entry has a small JSON file with its text. Its
`files` list gives the tabs in display order: `name` is the label the site
shows, and `code` names the file:
`{ "file": "server/controllers/ItemsController.py" }`.
`{ "include": "streaming/section.json" }` reads another JSON file. Paths are
relative to the JSON file and stay inside `content/`.

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
follow the light and dark themes. Tailwind is imported without Preflight,
because DuVay owns the base layer.

## Sitemap and robots

The build writes `sitemap.xml` and `robots.txt` itself, because
`client/manifest.json` sets `tac.origin`. The sitemap lists the static pages
and, through `tac.sitemap`, one address for each guide in `docs.json` (`order`)
and each feature in `features.json` (`features[].id`), so it matches the site
that was built. Every page has a canonical URL and Open Graph and Twitter
metadata from `tac.metadata`.
