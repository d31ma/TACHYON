# Strict annotation modules in Tac and Yon

Role annotations define a source layout that the compiler enforces. For each
annotated type, these things are fixed:

- the type name's suffix;
- the exact file name;
- the folder directly above the file.

That folder is shared per realm: `client/<role>` and `server/<role>` (ADR 0031).
Controllers live in `server/controllers/`. Their annotations declare the URL (ADR 0048).
These are Tachyon rules, not Spring Boot requirements.

Data types without a role, such as TypeScript interfaces or Rust structs,
live in `client/models/` or `server/models/` (see
[Models hold types](#models-hold-types)).

## Role names and locations

| Role | Example type | Required file |
| --- | --- | --- |
| `RequestRoute` | `MoneyController` | `server/controllers/MoneyController.<ext>` |
| `Service` | `MoneyService` | `client/services/` or `server/services/MoneyService.<ext>` |
| `Store` | `MoneyStore` | `client/stores/` or `server/stores/MoneyStore.<ext>` |
| `HTTPClient` | `MoneyClient` | `client/clients/` or `server/clients/MoneyClient.<ext>` |
| `Helper` | `MoneyHelper` | `client/helpers/` or `server/helpers/MoneyHelper.<ext>` |
| `Converter` | `MoneySummaryConverter` | `client/converters/` or `server/converters/MoneySummaryConverter.<ext>` |
| `Setup` | `MoneySetup` | `client/setups/` or `server/setups/MoneySetup.<ext>` |
| `Entity` | `MoneyEntity` | `client/entities/` or `server/entities/MoneyEntity.<ext>` |

An authored file with a framework-annotated type holds exactly one class and
its imports (ADR 0044). The rule also covers plain Yon handlers, workers and
root middleware. A controller has the Controller suffix; a worker class is `Handler`. The class
of the root middleware file is `Middleware`, with `before(request)` and
`after(request)`; a `Handler` class there fails with `TY1310`. An extra
top-level type, function, variable, constant or statement fails with
`TY1310`, even when that extra declaration has no marker.
Nested types belong to their containing class. Rust may declare the type and
the `impl` blocks for that same type in one file.

Move pure data types into `models/` and plain helper code into `shared/`, using
the language's supported imports. A discoverable adapter or annotated binding
holder can be a separately named `Helper` in `helpers/`; for example,
`OrderInputHelper` can also carry `Checked`. Modifiers stay on the type
or member they describe, not on another helper class in the same file.

A behavior-only class such as a `CacheSettings` or `Settings`
holder can instead have its own file in an existing role folder such as
`helpers/`. It is composed without being registered for dependency injection;
only a primary role such as `Helper` adds that role. Unannotated helpers
are not automatically composed. Included role files still reject relocatable
relative imports; use the existing supported imports rather than placing
imports alone in an otherwise empty route marker.

Old `yon.*` route markers and `OPTIONS.schema.json` are refused. A controller
is its own entry point. External relay and command-gateway programs are not handler files and
remain outside this rule.

The compiler validates authored files before combining them. Generated units
may contain multiple framework-owned types; the rule does not run again on
that generated output.

A test unit holds only its `Test` class. Put its `TestSetup`
class in a sibling file whose name matches the class. The unit's
`configuration` option selects that exact file; unrelated test setups are not
loaded, and test setups never enter production bundles.

Naming rules:

- A type name ends with the canonical role name and starts with a domain
  prefix.
- The file name stem matches the complete type name exactly, including case.
  This is true even on case-insensitive filesystems.
- Rust `impl` declarations belong to their target type.
- You cannot solve conflicting roles by joining suffixes. Split the
  responsibilities instead.

RequestRoute exists only in Yon, and so does Stream. Every other role exists in
both realms (ADR 0031).

- A Yon service is a provider in `server/services`. Every route shares it.
- A Tac service is the behaviour that a component view names. It lives in
  `client/services` (ADR 0030).
- Lifetime applies to both. A Tac service is a page singleton by default.
  `@Lifetime({"value":"prototype"})` creates one for each component instance.
- A Tac Helper is an injectable role with no process rights.
- A Tac Helper and a Tac Client are providers. They are injected into
  services through NeedsAll. A Tac Store is a contract that names its Entity;
  a Tac service builds it (ADR 0022 amendment).
- HTTPClient also supports automatic browser Tac JS/TS construction. A client
  is a contract with no implementation (an interface, trait, protocol,
  Protocol class, or a class with empty methods in JavaScript, TypeScript and
  Ruby). Tachyon generates it; a native Tac service builds it, and its Setup
  configures the HTTP adapter.
- Object-form query Store, Setup and Entity are shared.

These rules do not enable a role on a framework or target that does not
support it.

Modifiers do not change naming or placement. Examples of modifiers: Needs,
Lifetime, CacheSettings, NeedsAll, Table, WhenProfile, WhenSetting, Settings,
Checked, field Setting, OnEvent, OnStart, OnStop, Order and InBackground. MoneyEntity may also carry Table.
A modifier does not require another suffix. When a binding holder also uses
the discoverable `Helper` role, its class and filename end in `Helper`.

## Required fields, not handwritten constructors

Managed roles can add NeedsAll() and declare direct fields that
are not initialized and not static. The compiler infers provider tokens and
generates the assignments. You do not need an authored `Needs` list or
constructor.

- Typed languages use exact concrete providers. Rust uses Arc<Provider> or
  std::sync::Arc<Provider>.
- JavaScript matches exact lowerCamel provider names, such as MoneyService to
  moneyService and HTTPClient to httpClient.
- Declaration order is the default. An optional quoted field list selects
  dependencies or changes their order, without a `Needs` list.
- Initialized defaults and static fields do not change.
- A selected dependency fails closed if it is missing, ambiguous, optional,
  generic, an interface or a function.
- A Rust leaf uses a braced struct and an empty NeedsAll marker
  to generate new(). It does not use an authored constructor.

Constructor inference uses the entry scope that annotation discovery found.
NeedsAll is a modifier, so MoneyController still belongs in
server/controllers/MoneyController.<ext>.

Tac and unmanaged behavior types keep explicit construction, including
Behaviors.construct(Type, values). Setting and configuration binding stay
explicit. Settings holders and behavior contexts do not become providers
automatically. See [ADR 0025](adr/0025-required-constructor-inference.md).

## Entry points and annotation discovery

```text
client/
  pages/money/
    tac.html
  components/money/panel/
    tac.html
    tac.css
  services/MoneyService.ts
  setups/MoneySetup.ts
  entities/MoneyEntity.ts
  models/MoneyTotal.ts
server/
  services/MoneyService.ts
  stores/MoneyStore.ts
  controllers/MoneyController.ts
```

Tac:

- A Tac page under client/pages holds only a composing tac.html.
- A component under client/components holds its view and an optional tac.css.
- Behaviour is a `<Name>Service` class in client/services. The view addresses
  it by its instance name.
- Tac has no SSR (server-side rendering) mode.

Yon:

- Yon controllers live in server/controllers/.
- RequestRoute and method route annotations declare the URL and HTTP verb.
- Yon has no yon.html and no template rendering.

The role folders mirror each other:

| Client realm | Server realm |
| --- | --- |
| client/services | server/services |
| client/clients | server/clients |
| client/stores | server/stores |
| client/setups | server/setups |
| client/entities | server/entities |
| client/helpers | server/helpers |
| client/converters | server/converters |

Every Yon entry composes the shared roles of its language and its own
controllers. A shared service is one class. Each invocation still owns its
graph of instances.

A role file beside a route fails with TY1310, "@Service type 'X' sits outside
its realm's shared role folder". The message names its shared destination.
`ty migrate` reports it as `role.route` Changed, with the same destination.

Within the bounded role folders, the compiler discovers role-bearing and
behavior-only annotated files that match the entry's language. It reads them
in stable path order. Behavior-only annotations do not register DI providers.

Services are files directly below client/services. Each one is named
`<Name>Service.<ext>` and has a class with the same name. These cases fail
with TY1310:

- a subdirectory;
- a file with the wrong name;
- a service without a default export;
- a service that declares both a js and a ts file.

No module manifest is required or accepted.

| Realm | Languages |
| --- | --- |
| Yon | js, ts, py, php, java, kt, cs and rs |
| Tac | js, ts, swift, kt, cs and rs |

The existing target matrix still applies, per service:

- JS/TS on every target;
- Swift on Apple;
- Kotlin on Android;
- C# on Windows;
- Rust on desktop.

Swift is not a Yon language. A native Tac service is
`client/services/<Name>Service.swift`, `.kt`, `.cs` or `.rs`. Its class is
named after the file, and it keeps no-argument construction.

The compiler takes the entry point and the discovered role files from one
immutable project snapshot. It assembles them into one compilation unit for
that language.

- Inside that unit, refer to module types directly by name. Do not use imports
  between selected modules.
- Ordinary relative imports in included modules currently fail closed.
- Annotations and framework types need no import. Supported nonrelative
  native or standard-library imports are still allowed.
- This adds no general package resolver.
- Direct type references at definition time in JS/TS/Python are ordered by
  dependency.
- Dynamic initialization hidden inside calls or interpolated strings is not a
  general dependency graph. Keep cross-file initialization in methods that run
  after construction.
- The compiler rejects an own-line `static field =` initializer that uses a
  directly interpolated template literal, with this guidance. That check does
  not reject method strings, instance fields or deferred arrow-function
  bodies.

Role-bearing declarations cannot stay inline in tac.* or yon.*, because those
reserved file names cannot match a role type. A Yon entry point may contain
one of these:

- an ordinary Handler that uses discovered types;
- only a comment, when a discovered controller owns the verb methods.

HTTP method decorators are still not needed. Unannotated data-only DTOs belong
in models. Annotated validation and property schemas have their own named
files in a supported folder; they do not share the handler or primary role's
authored file.

## Models hold types

`client/models/` and `server/models/` hold the data types that services and
other files share. A models folder is not a role folder. A models file holds
data, not behavior (ADR 0041, ADR 0043).

Name the file after the type it holds, or after a small group of related
types: `client/models/Guide.ts`, `server/models/Guide.py`.

### Which languages a models folder holds

| Realm | Languages |
| --- | --- |
| `client/models/` | TypeScript, Rust, C#, Dart, Kotlin, Swift, Go |
| `server/models/` | TypeScript, Rust, C#, Dart, Kotlin, Java, Python, PHP, Go |

Swift is not a Yon language, so only `client/models/` holds Swift files.
Java, Python and PHP run only in Yon, so only `server/models/` holds their
files. JavaScript has no type declarations: it describes types with JSDoc in
the file that uses them.

### What a models file may declare

| Language | Allowed |
| --- | --- |
| TypeScript | `interface`, `type` aliases and `import type`, with or without `export` |
| Rust | `struct`, `enum` and `type` aliases, their attributes such as `#[derive(...)]`, and `use` declarations |
| C# | `record`, `class` and `struct` declarations with only fields and auto-properties, `enum`, their attributes, and `using` directives |
| Dart | `class` declarations with only fields and constructors without a body, `enum`, `typedef`, and `import` directives |
| Swift | `struct` with only stored properties, `enum` with only cases, `typealias`, compiler-synthesized conformances such as `Codable`, `Equatable` and `Hashable`, and `import` declarations |
| Kotlin | `data class` and `class` with only properties, `enum class` with only entries, `typealias`, their annotations such as `@Serializable`, and `import` directives |
| Java | `record` declarations with an empty body, `enum` with only constants, `class` with only fields, their annotations, and `import` declarations |
| Python | `@dataclass`, `TypedDict`, `NamedTuple` and `Enum` classes with only annotated fields, default values or members, and `import` statements |
| PHP | `class` with only typed properties (a constructor that only promotes properties, with an empty body, is allowed), `enum` with only cases, and `use` statements |

A models file may not contain methods, functions, computed properties,
property bodies with logic, operators, traits or interfaces with behavior,
top-level variables or constants, service or role markers, or any other
statement. Python type aliases are not allowed either, because a module-level
alias is a file-level variable.

### How other files use a model

| Language | Import |
| --- | --- |
| TypeScript | `import type { Guide } from '/models/Guide.ts'` |
| Rust | `use crate::models::Guide;` |
| C# | `using Models;`, then `Guide` |
| Dart | `import 'models/Guide.dart';` |
| Swift | `Guide`, with no import |
| Kotlin | `import models.Guide` |
| Java | `import models.Guide;` |
| Python | `from models import Guide` |
| PHP | `use Models\Guide;` |

- TypeScript removes its type-only import, so the browser and the server
  never load a TypeScript models file. `/models/` is the realm's models
  folder, as `/shared/` is its shared folder.
- In the other languages, the framework adds the realm's models files of that
  language to every program it builds from that realm's code in that
  language: native companions, WebAssembly and Dart browser services, Yon
  handlers, the project-root middleware (`middleware.py`, `middleware.php`)
  and test units. A `middleware.ts` uses a type-only import from `/models/`,
  as every TypeScript file does.
- Rust models become the module `crate::models`, so declare the type and its
  fields `pub`.
- Dart models sit in a `models/` folder next to every Dart program that the
  framework compiles, so `import 'models/Guide.dart';` works in both realms.
  One Dart models file imports another by its file name:
  `import 'Section.dart';`.
- A models file is never discovered as a service, never routed and never
  shipped as its own file.

```ts
// client/models/Guide.ts
import type { Section } from '/models/Section.ts'

export interface Guide {
  title: string
  sections: Section[]
}
```

```rust
// client/models/Guide.rs
#[derive(Clone, Debug)]
pub struct Guide {
    pub title: String,
    pub pages: i64,
}
```

```rust
// client/services/GuideService.rs
use crate::models::Guide;

#[Service()]
#[derive(Default)]
struct GuideService {
    pages: i64,
}

impl GuideService {
    fn title(&mut self) -> String {
        let guide = Guide { title: String::from("Models hold types"), pages: 3 };
        self.pages = guide.pages;
        guide.title
    }
}
```

```python
# server/models/Guide.py
from dataclasses import dataclass


@dataclass
class Guide:
    title: str
    pages: int = 0
```

These cases fail with TY1310:

- A role annotation, such as `@Service`. The message names the role folder
  that the type belongs in, for example `client/services/GuideService.ts`.
- A method, a function, a computed property, a constant, a service marker
  such as `@Lifetime`, or any other statement. The message says where the code
  belongs: a service or role file for behavior, or `client/shared/` or
  `server/shared/` for plain code.
- A file in a language that the realm's models folder does not hold, for
  example `server/models/Guide.swift` or `client/models/Guide.py`.

## Bounded source discovery, not runtime scanning

Role annotations trigger validation and registration within their entry scope.

- Project discovery checks annotated files in its captured source trees before
  build or execution. This includes interpreted Yon languages.
- A wrong type suffix, file name or folder produces TY1310 without running the
  application.
- A direct handler invocation also validates the source before it starts its
  runtime.

The compiler scans the realm's six shared role folders and their descendants.
For a Yon entry, it also scans the controllers folder beside that route. It
never crosses a nested Tac or Yon entry boundary and never follows a symlink.

Discovery limits per entry:

| Limit | Value |
| --- | --- |
| Annotated files | 128 |
| Directory entries | 4096 |
| Nested directories | 32 |
| Same-language source | 1 MiB |

Files without annotations do not register providers, but their bytes still
count toward the scan budget. Discovery does not follow imports, does not mix
languages and does not create a global container. Ordinary helpers without
annotations keep the ordinary composition rules of their language. They do
not need role folders.

Names are unique within the assembled unit. Generated metadata and contexts
belong to that unit. You cannot pass them between independent generated units.
Client entry scopes contain client/public code, never server types that hold
secrets. Share wire contracts and tests without bundling private runtime
configuration.

## Migration checklist

1. Identify the primary roles. Leave modifiers attached to their owner.
2. Rename these things: role types, dependency fields and types, any explicit
   `Needs` tokens you keep, Store entity strings and explicit behavior
   factory arguments.
3. Move each type into the exact plural folder, with the matching file name.
4. Move role folders out of route directories into client/<role> or
   server/<role>. Leave controllers beside their route. Remove old
   tac.modules.json and yon.modules.json files, because they fail with TY2101.
5. Keep the route or page entry point, and rebuild to validate discovery.
6. Split helper declarations out of annotated and handler files. Keep nested
   types inside their owning class; move independent data into models and
   behavior into its own role or shared module.
7. Move each inline TestSetup into its own sibling test file, keeping
   the Setup name selected by `Test` unchanged.
8. Run `ty migrate fix --write` to rename annotations, role folders and role
   types from before [ADR 0047](adr/0047-annotation-names-are-plain-english.md)
   (for example `Repository` to `Store` and `repositories/` to `stores/`).

Examples:

- An inline @Service class Money becomes server/services/MoneyService.ts or
  client/services/MoneyService.ts.
- An old @Controller class MoneyApi becomes a @RequestRoute("/money") class in
  server/controllers/MoneyController.ts and injects MoneyService.
- A renamed entity also requires Store(MoneyEntity, {...}).

Keep HTTP methods small. A method should authenticate, validate input, create
an explicitly owned context, call services and return a response.

- Caching still requires authorization.
- Query adapters own parameterization and tenancy.
- Configuration binds TAC_ or YON_ values that are supplied explicitly.

Layout validation does not add transactions, ORM migrations, database
drivers, implicit configuration binding or instance reuse across requests.

See [behavior annotations](BEHAVIOR_ANNOTATIONS.md),
[Yon annotations](YON_ANNOTATIONS.md),
[ADR 0024](adr/0024-strict-annotation-modules.md) and
[ADR 0031](adr/0031-realms-mirror-each-other.md).
