# ADR 0092: Settings and Validation Data Live in Models

- Status: Accepted
- Date: 2026-10-01
- Amends: [ADR 0043](0043-models-hold-data-types.md) (models may carry
  declarative settings and validation metadata)
- Related: [ADR 0091](0091-role-folders-allow-nesting-and-require-explicit-registration.md)
  (a concrete class in a role folder needs its role marker)

## Context

ADR 0091 requires a role marker on every concrete class in a role folder.
That check found settings and validation data classes in `helpers/`. These
classes describe data. They should not get a Helper lifecycle, and they are
not services.

## Decision

Settings and validation data holders live in the realm's `models/` folder.
A model may carry declarative `Settings`, field `Setting` bindings, `Checked`
schemas, the field constraints `Required`, `NotEmpty`, `Length`, `Min` and
`Max`, and `Description` metadata. This metadata does not register a Helper
or a Service. Native annotations stay native. The existing behavior parser
checks where the schema annotations are placed and that their arguments are
literals. Other framework roles and behavior annotations stay refused, as do
methods and constructors.

Classify each moved declaration by what its code does:

| Authored declaration | Destination |
| --- | --- |
| `ApplicationValues` fields with `Setting` metadata | `models/`, with configuration metadata |
| `ApplicationSettings` or `Settings` fields and constraints | `models/`, with settings metadata |
| `ProductInput` or `Input` fields with `Checked` and constraints | `models/`, with validation metadata |
| A stateful `InputService` with `calls` and a `Check`-guarded `submit` method | `services/`, with `Service` |
| A real helper collaborator with behavior methods | `helpers/`, with `Helper` and the helper naming rules |

Keep data type names and the references to them when you move a model. Add
explicit imports where a language needs them. Do not add a Helper marker to a
data holder, and do not rename a Service into a Helper. The historical neutral
and release fixtures stay unchanged; compatibility tests migrate generated
copies with explicit path and role mappings.

A C# schema class may keep a private constructor without parameters whose
whole body throws `Exception` or `System.Exception` with a plain string
literal. This keeps the existing fixture that must never be instantiated.
Interpolated or computed arguments, assignments, method calls and other
constructor bodies are refused. Metadata lowering reads declarations and
never constructs these classes; SDK output keeps the authored constructor.

### Yon

Yon composes the authored schema declarations before behavior lowering.
Lowering captures the configuration and validation metadata and then removes
the temporary data declarations. The model projection emits each model once,
without the consumed schema annotations, and keeps imports, namespaces and
native type declarations. Rust, C#, Kotlin, Python and PHP code reaches models
through explicit imports in their model namespaces. Java and Groovy stay in
their flattened compilation unit, Go stays in package `main`, and Ruby models
are ordinary inline constants. Lua schema models are refused with `TY1310`;
plain Lua models and Lua converters stay supported.

### Tac

In Tac, JavaScript, TypeScript, Java and Python read these schemas. JavaScript and
TypeScript schema classes hold data fields and literal defaults; constructors,
methods, getters, inheritance and computed defaults are refused. JavaScript
refers to them with JSDoc types, and TypeScript with type-only imports of the
declared schema name. Renamed schema imports and runtime value imports from
`models/` are not supported. Valid input reaches the service; invalid input
fails before the service method runs. Ordinary JavaScript JSDoc models and
TypeScript interfaces are unchanged.

The compiled Tac languages read these schemas too; see the amendment below.
Lua schema models fail with a located `TY1308` or `TY1310` instead of being
accepted with their checks silently dropped. Plain client models stay supported in every Tac
language.

## Amendment (2026-10-03): every compiled Tac language reads schema models

Tac reads schema models in C#, F#, Rust, Swift, Kotlin, Go, Dart and Ruby as
well. The build works the same way as in Yon. Before it lowers a service, it
joins the authored schema declarations from `client/models/` to the service
text. After lowering, it removes that temporary copy. The model then compiles
once, from its file in `client/models/`, without the consumed markers. The
compiler's service check and the C# browser factory read the same joined
text. A Rust browser model check reads it too.

A service reaches a schema model the way it reaches any model in its
language:

| Language | How the service names the model |
| --- | --- |
| C#, F# | `using Models;` / `open Models` |
| Rust | `use crate::models::Name;`, with `pub` on the struct and its fields |
| Kotlin | `import models.Name` |
| Dart | `import 'models/Name.dart';` |
| Swift, Go, Ruby | no import |

A Swift schema model is a `struct`, as every Swift model is. The behavior
reader now accepts a Swift `struct` as a type that carries schema markers. A
Kotlin schema model keeps its fields in the class body. It is not a browser
DTO, so the Kotlin DTO checks and readers leave it out. Only Lua schema
models are still refused (`TY1308` for `Checked`, `TY1310` otherwise).

The same examples needed two Yon fixes:

- Dart: the generated reader for a model with no constructor called it with
  named arguments, and the handler did not compile. The reader now creates
  the model with its default constructor and assigns each field that is not
  `final`.
- Groovy: an import line has no semicolon. The model block reader joined the
  hoisted `import models.X` line with the first schema model. It then could
  not remove the lowered copy, and `Program.groovy` declared the class twice.
  Groovy top-level statements now end at a line end, as in Kotlin and Swift.

Rust Yon needed no change. A schema model is reached with
`use crate::models::{Name}` and must be `pub`, like every Rust model.

## Amendment 2026-10-17: Tac Lua schema models

A Lua schema model in `client/models/` is now read like the other Tac
languages. Its markers are `--` comments, so the model table runs unchanged,
and the service program reads the schema from the Ruby shadow. Yon Lua schema
models stay refused with `TY1310`. See ADR 0085.
