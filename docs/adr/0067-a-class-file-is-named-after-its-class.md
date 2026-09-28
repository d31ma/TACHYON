# ADR 0067: A Class File Is Named After Its Class

- Status: Accepted
- Date: 2026-09-27
- Preserves: [ADR 0031](0031-realms-mirror-each-other.md) (role folders per
  realm), [ADR 0043](0043-models-hold-data-types.md) (models hold data types),
  [ADR 0044](0044-an-annotated-file-holds-one-class.md) (a file holds one
  class) and [ADR 0064](0064-code-is-not-duplicated.md) (`<Topic>Utils`
  utility files)

## Context

Role files (`TY1310`) and utility files (ADR 0064) already had to carry the
name of their class. Other class files did not. `server/models/Order.ts` could
declare `Invoice`, and `client/tests/ShellServiceTest.js` could hold
`class ShellChecks`; both built and ran. A reader then had to open a file to
learn which class it held, and a search by class name missed it. Java solves
this with one rule: the file name is the name of its public class. The shared
engineering instructions ask for the same rule in Tachyon's own code.

## Decision

1. **The rule.** In a Tachyon project, every file in `client/` or `server/`
   that declares a class, struct, record, interface, object, protocol, trait or
   model type is named after that type. The file name without the extension is
   the type name with the same spelling and case, in every language:
   `CartService.ts`, `Order.py`, `Order.rb`, `CartServiceTest.go`. Python and
   Ruby follow the PascalCase file names that Tachyon already uses for their
   role files, so each language has one naming style.
2. **What it covers.** Role files, models, test units, test setups,
   utilities, error types, delegates and every other class file in both
   realms. A models file must declare the model that it is named after.
3. **What keeps its name.** Views and component files (`tac.html` and its
   `tac.*` companions) in `client/pages/` and `client/components/`; plain code
   in `client/shared/` and `server/shared/`; workers in `server/workers/`,
   whose class is `Handler` and whose file name is the worker's name; and the
   root `middleware.<ext>`, whose class is `Middleware`. A file that declares
   no type, such as a Python script with functions only, has nothing to
   follow. The code that creates the framework's annotations is framework
   code, not project code.
4. **The error.** A file with another name fails the build with `TY1318` in
   the `TY1310` family, before the compiler composes the program. The message
   names the file, the class and both fixes: rename the file or rename the
   class.
5. **The migration.** `ty migrate fix --write` renames the file after its
   class when the file declares exactly one type. A rename that changes only
   the case of letters works on file systems that ignore case. When another
   file uses the old file name, for example a relay command such as
   `server/delegates/report.rb`, the file name is the reference, so the fix
   renames the class in that file instead, if the file name is a valid type
   name and no other file uses the class. Otherwise it lists a manual step.
6. **Tachyon's own Rust code.** `tools/cohesion-scan` counts every Rust file
   whose stem is not the snake_case name of its type (`route_state.rs` holds
   `RouteState`). `mod.rs`, `lib.rs`, `main.rs`, `build.rs`, files with no
   type and test files are not counted. The repository test keeps the count at
   or below `tools/cohesion-scan/FILE_NAME_BASELINE` (162 on 2026-09-27); new
   files follow the rule, and later refactors lower the baseline to 0.

## Consequences

- The file tree lists the classes of an app, and a search for a class name
  finds its file.
- The build now refuses projects that were valid before. The first
  `ty migrate fix --write` renames most files; a second run renames files that
  another fix edited in the first run.
- A models file holds one model since the amendment below.

## Amendment: one model per models file (2026-09-28)

The first version let a models file declare helper types next to its main
model, because the model readers did not yet name every declaration form.
They do now: one reader names the top-level models of a models file in every
language (TypeScript interfaces and type aliases, Rust structs, enums and
`type` aliases, C# records, record structs, classes, structs and enums, Java
records, classes and enums, Kotlin classes, enum classes and `typealias`,
Swift structs, enums and `typealias`, Dart classes, enums and `typedef`, PHP
classes and enums, Python classes, Ruby `Data.define`, `Struct.new` and
classes, Go types) and ignores nested types, imports and comments.

1. **The rule.** A file in `client/models/` or `server/models/` declares one
   model, and the file is named after it (Decision 1). A second model, even a small
   enum, fails the build with `TY1318` before the compiler composes the
   program. The message names every model in the file and the file for the
   second one.
2. **The migration.** `ty migrate fix --write` splits the file. The model
   that the file is named after stays; if no model has the file's name, the
   first one stays and a second run renames the file. Each other model moves
   to `<Model>.<ext>` in the same folder with the file's imports and its own
   comments, attributes, decorators and YARD types. In TypeScript, Rust and
   Dart, where a model is visible only through an import, each file gets an
   import of the other models that it uses (`./Order.ts`,
   `crate::models::Order`, `Order.dart`), and TypeScript, JavaScript and Dart
   files that imported a moved model from the old file import it from the new
   one. When a target file already exists, or an importer is changed by
   another fix, the fix lists a manual step instead.
3. **JavaScript.** Amended 2026-09-28: JavaScript has models files too. A
   JavaScript models file (`client/models/CartTotal.js`) holds one JSDoc
   `@typedef` and `export {}`, follows the one-model and file-name rules, and
   is a global type of its realm through `composed.d.ts`. JavaScript routes and
   listeners read the JavaScript and TypeScript models of their realm, so a
   JavaScript file uses a TypeScript model with
   `/** @import { PlatformChanged } from '/models/PlatformChanged.ts' */`
   instead of copying it. A `@typedef` in another JavaScript file of a realm is
   a model outside its folder: `ty migrate fix --write` moves it, keeps one
   file for identical copies, imports an existing model that the copy matches,
   and lists a manual step for a copy that differs. The build rule that refuses
   an inline `@typedef` (TY1310) stays off until the website examples, fixtures
   and templates follow it.
