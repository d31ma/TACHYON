# ADR 0053: Composed Classes Have Generated TypeScript Globals

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0046](0046-tachyon-annotations-are-global.md) (the ambient
  declarations for editors and `tsc` now also cover the application's own
  composed classes, and they are generated)
- Preserves: the composition rules of
  [ADR 0031](0031-realms-mirror-each-other.md) and
  [ADR 0044](0044-an-annotated-file-holds-one-class.md), the compile-time
  model, and the rule that a role file needs no import of another role

## Context

Tachyon composes role classes into the programs that use them. A test in
`client/tests/` can write `new TopicService()` with no import, because the
compiler adds `client/services/TopicService.ts` to the test's program. In
Python, PHP, Java, Kotlin, C#, Rust, Swift, Dart and Go, the language compiler
sees only the composed program, so this works in editors too.

JavaScript and TypeScript editors and `tsc` read each file alone. They did not
know `TopicService`. The website kept a hand-written file,
`website/composed-services.ts`, that declared three services as globals. It
had to change whenever a service changed, and nothing checked that it matched
what Tachyon composes.

## Decision

- `ty` writes `.tachyon/types/client/composed.d.ts` and
  `.tachyon/types/server/composed.d.ts` in the project. `ty bundle`,
  `ty serve`, `ty preview`, `ty test` and `ty init` write them, and every
  rebuild in watch mode writes them again. The new command `ty types` writes
  only these files. `.tachyon/` is not committed.
- Each file declares, as TypeScript globals, the JavaScript and TypeScript
  classes that Tachyon composes by name in that realm:
  - the annotated classes in the realm's role folders (`clients`, `setups`,
    `delegates`, `entities`, `converters`, `stores`, `services`), found the
    way the compiler finds them: same folders, same nesting and entry rules,
    no symlinks, and only files with a Tachyon annotation;
  - on the server, also the error types in `server/errors/` and the
    `*ErrorHandler` classes in `server/controllers/`, which route controllers
    compose.
- Each file also declares the `@TestSetup` classes in the realm's `tests/`
  folder, because a test unit names its configuration in `@Test(...)` and
  `tsc` must find that name.
- Controllers, models, test units and plain helper files are not declared.
  Tachyon does not compose them by name: models are imported.
- A class is declared only when its file exports it (`export class` or
  `export default class`). The global has the class's real type:
  `type X = import("…").default` and `const X: typeof import("…").default`.
  A file with no import or export is already a global script, so it needs no
  declaration.
- The file is written again from the sources each time. A deleted class
  disappears from it. A realm without classes gets a file with only
  `export {};`. The file is not rewritten when its text is unchanged.
- The realms stay apart. A client class is never declared for server files,
  and the other way round, because mirrored realms often use the same names
  (ADR 0031). `ty init` therefore writes one `jsconfig.json` per realm,
  `client/jsconfig.json` and `server/jsconfig.json`, instead of one at the
  root. Each includes `../tachyon-env.d.ts` and its own
  `../.tachyon/types/<realm>` folder.
- If `.tachyon/types` cannot be written, for example because it is a symlink,
  the command stops with `TY1410`. Nothing is written outside the project.

## Consequences

- JavaScript and TypeScript authors use composed classes without an import,
  with full types, in editors and in `tsc`, like authors in the other
  languages.
- The website deletes `website/composed-services.ts`. The root
  `jsconfig.json` includes `website/.tachyon/types/client`, and CI runs
  `ty types website` before `bun run typecheck`.
- TypeScript cannot give each folder its own globals. Inside one realm the
  declarations are the union of what every file kind sees. The compiler is
  stricter: a service does not see other services, and a JavaScript file does
  not see a TypeScript class. `tsc` accepts these uses, but they fail at run
  time.
- A project must run one `ty` command before its first type check.
