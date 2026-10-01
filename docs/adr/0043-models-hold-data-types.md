# ADR 0043: Models Hold Data Types in Every Service Language

- Status: Accepted
- Date: 2026-09-24
- Amends: [ADR 0041](0041-models-hold-types.md) (a models file was
  TypeScript only)
- Preserves: the TypeScript models rules, every role folder, and the rule
  that a service's own type and its behavior live in the service's own file
- Amended by: [ADR 0044](0044-an-annotated-file-holds-one-class.md), which
  moves every type without a role out of annotated files and into `models/`

## Context

ADR 0041 gave TypeScript a place for types without a role:
`client/models/` and `server/models/`. The other service languages still had
to declare their data types inside a service or role file. So two services could not
share one data type, and a reader had to skip the type to find the service.

A service's own type is not such a data type. A Rust service struct, and a
service class in the other languages, carry the service markers, and their
fields are the service's state. The framework reads that type and its methods
together to build the service. So the service type stays in the service's
file.

## Decision

- `client/models/` and `server/models/` may also hold `.rs`, `.cs`, `.dart`
  and `.kt` files with data types. Only `client/models/` may hold `.swift`
  files, because Swift is not a Yon language. Only `server/models/` may hold
  `.java`, `.py` and `.php` files, because those are Yon-only languages.
- A models file holds data, not behavior:
  - Rust: `struct` and `enum` declarations and `type` aliases, their
    attributes (such as `#[derive(...)]`), and the `use` declarations they
    need.
  - C#: `record`, `class` and `struct` declarations with only fields and
    properties (including positional records and auto-properties), `enum`
    declarations, their attributes, and the `using` directives they need.
  - Dart: `class` declarations with only fields and constructors, `enum`
    declarations, `typedef`s, and the `import` directives they need.
  - Swift: `struct` declarations with only stored properties, `enum`
    declarations with only cases, `typealias`es, conformances that the
    compiler synthesizes (such as `Codable`, `Equatable`, `Hashable`), and
    the `import` declarations they need.
  - Kotlin: `data class` and `class` declarations with only properties,
    `enum class` declarations with only entries, `typealias`es, their
    annotations (such as `@Serializable`), and the `import` directives they
    need.
  - Java: `record` declarations without a body, `enum` declarations with
    only constants, `class` declarations with only fields, their
    annotations, and the `import` declarations they need.
    In Tac, a data-only class may use `@Checked` and its fields may use
    `@Required`, `@NotEmpty`, `@Length`, `@Min` and `@Max`. The owning
    Service uses `@Check` to enforce that schema. The compiler captures
    schema metadata before lowering and removes the schema annotations
    before javac, retaining one DTO declaration. Role annotations,
    behavioral methods and constructors remain forbidden in models.
    Checked Java models in Yon currently fail at their marker until the
    [server metadata adapter](../queue/NEXT/JAVA-YON-CHECKED-MODELS.md)
    is qualified; plain Java DTOs remain supported in both realms.
  - Python: classes that are `@dataclass`, `TypedDict`, `NamedTuple` or
    `Enum` classes, with only annotated fields, default values or members,
    and the `import` statements they need. Type aliases are not allowed,
    because a module-level alias is a file-level variable and the `type`
    statement needs a newer Python than the project supports.
  - PHP: `class` declarations with only typed properties (a constructor that
    only promotes properties, with an empty body, is allowed), `enum`
    declarations with only cases, and the `use` statements they need.
  In every language, a models file may not contain methods, functions,
  computed properties or logic in property bodies, operators, traits or interfaces with behavior,
  top-level variables or constants, service or role markers, or any other
  statement. Anything else fails with TY1310, and the message says where the
  code belongs: a service or role file for behavior, or `shared/` for plain
  code.
- A service imports a model in its language's usual way:
  - Rust: `use crate::models::Name;`. The models files become the module
    `crate::models`, so a Rust model declares its type and fields `pub`.
  - C#: `using Models;`, then the type name
  - Dart: `import 'models/Name.dart';`. The framework puts the realm's Dart
    models files in a `models/` folder next to every Dart program it
    compiles, so this one relative form works in both realms and in every
    build: browser services, Yon handlers and test units. A models file
    imports another models file by its file name: `import 'Section.dart';`
  - Swift: the type name alone, because every file of a Swift module sees
    the others' types
  - Kotlin: `import models.Name`
  - Java: `import models.Name;`
  - Python: `from models import Name`
  - PHP: `use Models\Name;`
  The framework adds the realm's models files of that language to every
  program it builds from that realm's code in that language: native
  companions, WebAssembly and JavaScript services, Yon handlers, the
  project-root middleware and test units. So `middleware.py` and
  `middleware.php` import a model as a handler does, and `middleware.ts`
  uses a type-only import from `/models/`.
- Models files are never discovered as services, never routed, and never
  shipped as their own files.
- The website shows the rule with an example in every language that has one.

## Consequences

- Services can share data types without repeating them, in every service
  language except JavaScript.
- A service's own type and its methods still live in its own file.
- Each models file check reads top-level declarations and the members of each
  type. It rejects every item kind that is not listed above.
- JavaScript has no type declarations. It describes types with JSDoc in the
  file that uses them.
