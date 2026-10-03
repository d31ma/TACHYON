# ADR 0064: Code Is Not Duplicated

- Status: Accepted
- Date: 2026-09-27
- Preserves: [ADR 0031](0031-realms-mirror-each-other.md) (the realms stay
  apart), [ADR 0044](0044-an-annotated-file-holds-one-class.md) (a file holds
  one class), [ADR 0046](0046-tachyon-annotations-are-global.md) (composed
  names need no import) and [ADR 0063](0063-services-and-controllers-are-cohesive.md)
  (methods are cohesive)

## Context

The same block of code was copied into several services and controllers,
often with other names. A fix in one copy missed the others. The shared
engineering instructions already forbid duplicated code in Tachyon's own
source. SonarQube finds such blocks with its copy-paste detection (CPD), but
Tachyon must not send application code to another service, and Tachyon owns
its rules, in the same way as it follows Spring for annotations.

## Decision

Tachyon runs its own copy-paste detection with SonarQube's semantics.

1. **What a duplicate is.** A block of at least 100 successive tokens that
   spans at least 10 lines and appears twice or more in the same language and
   the same realm (`client/` or `server/`), in one file or in two files.
   Names and literal values are normalized, so a copy with other names or
   values is still a duplicate. Keywords keep their spelling. Whitespace and
   comments are not tokens.
2. **What is not counted.** Imports, `use`, `using`, `require`, `package`
   and similar lines; annotation lines; files in `tests/`, `models/` and
   `entities/` folders (test units and model field declarations); files in
   `assets/` and `vendor/` folders and minified files (`.min.`), which are
   copied libraries; files with
   a test annotation; generated files (`.d.ts`, `.generated.`,
   `_generated.`, `.g.`); and a block that holds almost only values (fewer
   than one name or keyword in ten tokens), which is a list of data.
3. **The error.** `ty build`, `ty preview`, `ty serve`, `ty bundle` and
   `ty test` refuse a duplicate with `TY1315`. The message names both places
   as `file:first-last` lines, and the help says: "Move the repeated code
   into a utility: create `<realm>/utils/<Name>Utils.<ext>` with a static
   method, and call it from both places." `<Name>` comes from the words that
   the two file names share, without role words such as `Service`;
   otherwise it is `Shared`. `ty migrate check` lists every duplicate as a
   `duplication.block` finding.
4. **Utility classes.** `client/utils/` (Tac) and `server/utils/` (Yon)
   hold utility classes. A file there is named `<Topic>Utils.<ext>` and holds
   one class of the same name (`TY1310`) with no Tachyon annotation. Rust
   accepts the exact snake_case filename (`text_utils.rs` for `TextUtils`)
   alongside legacy `TextUtils.rs`, as amended in ADR 0067. The class
   holds only static methods and no state (`TY1316`): no field, no mutable
   storage, no constructor and no instance method. Rust utility impl blocks
   may hold immutable associated `const` items, including restricted visibility.
   These compile-time values are not instance fields or `static` storage. This
   2026-10-01 amendment follows the shared coding guide and still rejects
   instance fields, `static` and `static mut` (also inside a method), and
   receiver methods.
   Other languages' constant/field policies are unchanged. The methods follow the
   cohesion rule of ADR 0063. Tachyon composes every utility class of a
   realm into each program of that realm and language, with no import:
   services, controllers, error handlers, stores, test units and the other
   role classes. `composed.d.ts` declares the JavaScript and TypeScript
   utility classes, test units may test them, and `ty test --coverage` counts
   their public methods.

| Language | Utility class | Static method | Private helper |
|---|---|---|---|
| JavaScript | `export default class TextUtils` | `static slug(value)` | `static #lower(value)` |
| TypeScript | `export default class TextUtils` | `static slug(value: string)` | `private static lower(value: string)` |
| Python | `class TextUtils:` | `@staticmethod` above `def slug(value)` | `def _lower(value)` |
| Ruby | `class TextUtils` | `def self.slug(value)` | `def self._lower(value)` |
| PHP, Java, C#, Dart | `class TextUtils` (C#: `static class`) | `static` method | `private static` (Dart: `static _lower`) |
| Kotlin | `object TextUtils` | `fun slug(value: String)` | `private fun lower(...)` |
| Swift | `struct TextUtils` | `static func slug(_ value: String)` | `private static func lower(...)` |
| Go | `type TextUtils struct{}` | `func (TextUtils) Slug(value string)` | `func (TextUtils) lower(...)` |
| Rust | `pub struct TextUtils;` | `pub fn slug(value: &str)` (no `self`) | `fn lower(value: &str)` |

One switch per realm turns the duplication check off while a project
migrates, in the same place as the cohesion switch: `tac.duplication.check:
false` in `client/manifest.json` and `duplication.check: false` in
`server/application.json`. `ty migrate check` ignores the switch.

The detector is one small crate, `crates/tachyon-duplication`: a rolling hash
over the normalized tokens of each language finds equal windows of 100
tokens and stretches each match as far as it goes. The per-language token
reader is the one-class layout reader (`annotation_layout/single_class/tokens.rs`).
Tachyon's own Rust source uses the same crate through `tools/cohesion-scan`,
which now reports cohesion, structure and duplication together; the
repository test keeps the number of duplicated blocks at or below
`tools/cohesion-scan/DUPLICATION_BASELINE`, and the goal is 0.

## Consequences

- A fix to shared logic happens in one place, and the utility method has a
  name that says what it does.
- Two copies in different realms are not reported, because no utility class
  can serve both realms.
- The detector reads tokens, not a parse tree. A long list of similar
  statements in one file can repeat itself; move it into a loop over data or
  into `shared/data`.
- Rust utilities may keep immutable associated constants in their impl block.
  Other languages retain the existing field restriction. No mutable utility
  state is allowed.

## Amendment: Native Rust utility paths (2026-10-01)

Native Rust utilities remain validated plain classes in `client/utils/`, with
no Service or Helper marker. Canonical native assembly places them beside
the realm's other definitions. The compiler maps authored
`crate::utils::<KnownUtility>` imports and qualified references to that
type location, using the names of the checked utility classes.
It preserves grouped imports, aliases, and nested qualified calls without
changing application files or exporting a new root module. Comments, literals,
unknown names, module aliases and arbitrary external crate paths are unchanged.
An unqualified utility call keeps its existing common-module behavior.
Whole-folder aliases and globs, and namespace paths based on nested authored
directories, are not qualified by this per-type lowering. Nested code blocks
using a known fully qualified type are covered; nested directory namespaces
are a separate contract.

## Amendment: Rust declarations in the duplicate check (2026-10-01)

Behavior-clone tokens exclude top-level unit-type and impl wrappers and
associated constants initialized by a single literal value. Distinct label
tables followed by short accessors therefore do not become a behavior clone
merely because normalization replaces their different names and values.
Computed or block constant initializers, method bodies and function-local
declarations still count. Original source line positions and the existing
100-token, ten-line thresholds remain unchanged. Utility state and instance
method validation continues to inspect the complete authored source.

## Amendment: Tac utilities in client/shared/utils (2026-10-03)

Tac's canonical utility root is `client/shared/utils`, including nested topic
folders. `client/utils` remains a compatibility root. Yon keeps `server/utils`;
`server/shared/utils` is not a new implicit utility root. The same existing
stateless class, filename, immutable constant and role-marker checks apply.
There is no Utils marker and no utility exception for a Service or Helper.

JavaScript and TypeScript utilities may use explicit canonical absolute or
relative imports or the existing same-language implicit realm composition.
Utilities using `import.meta` preserve their original module identity and are
not also duplicated as inline class bindings. Named module publication and
editor declarations use the already validated utility class name. `.mjs` cannot
bypass the JavaScript/TypeScript utility shape checks.

The new folder reuses the existing utility shape checks; it adds no utility
support for F#, Lua or Groovy. Assets, models, Services and Helpers keep their
own folders.

## Amendment: client/utils is removed (2026-10-03)

This amendment replaces the compatibility root of the previous amendment. Tac
utility classes live only in `client/shared/utils`, and nested topic folders
are allowed. Discovery, composition, coverage and the service digest no longer
read `client/utils`. A file in `client/utils` fails with `TY1310`; the message
names `client/shared/utils`, and the help names the file's new path. The
duplicate-code help now names `client/shared/utils/<Name>Utils.<ext>` for Tac
and `server/utils/<Name>Utils.<ext>` for Yon.

`ty migrate fix --write` moves every file in `client/utils` to the same path
under `client/shared/utils`, in every language. It changes quoted import paths
that point at a moved file: relative paths, the absolute `/utils/` web path and
literal `client/utils/` paths. A relative path inside a moved file is computed
again from its new folder. A taken destination is a manual step. Native Rust
`crate::utils::<Utility>` paths need no change, because the compiler maps them
by the utility class name, not by its folder. Yon keeps `server/utils`
unchanged. The utility contract itself is unchanged.

As `client/utils` was, `client/shared/utils` is not copied as a raw shared
asset: the build publishes a utility module only when a script reaches it.
The design checks (visibility, layers, names) treat its files as utilities.
