# ADR 0044: An Annotated or Handler File Holds One Class and Its Imports

- Status: Accepted
- Date: 2026-09-25
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (what a role file may
  contain), [ADR 0041](0041-models-hold-types.md) and
  [ADR 0043](0043-models-hold-data-types.md) (where the other types go),
  [ADR 0021](0021-yon-annotation-frontend.md) and
  [ADR 0022](0022-shared-behavior-adapters.md) ("the same source file" now
  means the same composed unit), and the test unit layout
- Preserves: every annotation, every role folder, the `Handler` and
  `Middleware` entry points, programs called through `relay()` or a command
  gateway, and the data-only rules of models files
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them
- Amended by: [ADR 0048](0048-yon-routes-come-from-annotations.md): Yon routes come from annotations in `server/controllers/` (`RequestRoute`, `GetRoute`, ...), not from `server/routes/` folders; clients use `HTTPClient` and `GetRequest`, ...; `OPTIONS.schema.json` is replaced by schemas built from models
- Amended by: [ADR 0091](0091-role-folders-allow-nesting-and-require-explicit-registration.md): every concrete class in a role folder needs its role marker (`TY1363`), so a Tac service in `services/` can no longer leave out `@Service`; settings and validation data move to `models/` ([ADR 0092](0092-settings-and-validation-data-live-in-models.md))

## Context

A file that declares an annotated class could also declare more: helper
classes, helper functions, interfaces, type aliases, and a second annotated
class. A reader had to search the file to find the class that the framework
builds, and two files could hide the same helper in different places.

The project now has a place for each of those things. Types without a role
live in `models/` (ADR 0041, ADR 0043). Plain code lives in `shared/`. Every
role has its own folder (ADR 0031). So an annotated file no longer needs to
hold anything but its class.

Annotations are markers that the compiler reads and rewrites; they are not
runtime classes. So the rule is one shared check in the compiler, applied to
every annotated file and every handler file, rather than code in each
annotation.

## Decision

- These files hold exactly one class and the imports it needs:
  - a file whose class carries a framework annotation;
  - a Yon handler file: a route's `yon.<ext>`, a worker in `server/workers/`,
    and the project-root `middleware.<ext>`. The one class of a route or a
    worker is `Handler`. The one class of the root middleware is
    `Middleware`, with the methods `before(request)` and `after(request)`.
    Helper code moves to a service or to `shared/`.

  Nothing else may appear at the top level: no second class, function,
  interface, type alias, enum, constant, variable or statement.
- An empty or comment-only `yon.<ext>` route marker is allowed only when a
  controller for that route exists in `controllers/`. Without a controller,
  the marker fails with TY2015 because it does not declare a Yon Handler.
  Once the route file contains code, the one-class rule applies.
- Root middleware is a class named `Middleware`. The runtime calls its
  `before(request)` before the route and its `after(request)` after the
  route, for every request, whatever its HTTP method. A missing `before` or
  `after` lets that phase pass. A 204 answer continues (in the after phase,
  its headers are added to the response); any other status answers the
  request or replaces the response. A class named `Handler` in a middleware
  file fails with TY1310, and the message says that root middleware declares
  class `Middleware` with `before(request)` and `after(request)`.
- What counts as the one class, its imports and the few other lines a file
  may hold, per language. These lines are accepted:
  - JavaScript and TypeScript: one `class` (usually `export default class`),
    with its decorators, and `import` and `import type` statements. A shebang
    (`#!`) first line and a `"use strict";` directive as the first statement
    are also accepted.
  - Python: one `class` with its decorators, and `import` and `from ...
    import` statements. A module docstring as the first statement and
    `if TYPE_CHECKING:` blocks that hold only imports are also accepted. A
    line that ends with a backslash continues on the next line.
  - PHP: the opening `<?php` tag, one `class` or `interface` with its
    attributes, and `use` statements, including `use function` and
    `use const`. `declare(...);` statements, such as
    `declare(strict_types=1);`, and a closing `?>` at the very end of the
    file are also accepted.
  - Java: an optional `package` declaration, `import` statements, and one
    `class`, `interface` or `record` with its annotations. `package` and
    `import` lines run no code, so a Java route marker that holds only them
    counts as empty.
  - Kotlin: an optional `package` line, `import` directives and one `class`,
    `interface` or `object` with its annotations. A `companion object` and nested types stay inside the
    class. A primary constructor may start on the next line, with its
    annotations and visibility (for example `@Inject constructor(...)`).
  - C#: `using` directives and one `class`, `interface`, `record` or
    `struct` with its attributes. A file-scoped `namespace A.B;` line, or one
    block `namespace A.B { ... }` that holds the `using` directives and the
    one class, is also accepted, and so are `#nullable enable`, `disable`
    and `restore` lines.
  - Dart: an optional `library;` (or `library name;`) directive, `import`
    directives and one `class` with its annotations. The Dart 3 class
    modifiers `abstract`, `base`, `interface`, `final`, `sealed` and
    `mixin class` are accepted.
  - Swift: `import` declarations, including kind imports such as
    `import struct Foundation.Date`, and one `class`, `struct` or `protocol`
    with its attributes.
  - Rust: `use` declarations with any visibility (`pub use`,
    `pub(crate) use`), inner attributes such as `#![allow(dead_code)]`, and
    one `struct` (or `enum` or `trait`) with its attributes, plus the `impl`
    blocks of that same type.
  Types nested inside the class are part of the class. Comments may appear
  anywhere. A JavaScript or TypeScript class header may name its base class
  with a plain or dotted name and type arguments, such as
  `extends app.Base<T>`; a Python class header may name its bases, type
  arguments and keywords such as `metaclass=Meta`. In JavaScript,
  TypeScript and Python, a decorator argument may hold only strings, numbers,
  `true`, `false`, `null` (`True`, `False`, `None`), arrays and objects of
  those, and class names or dotted names, as in `@Inject(Counter)` and
  `@Cacheable({"cacheNames":["products"]})`. In the other languages the
  compiler already requires constant annotation arguments. The check reads strings as the language does, so braces inside
  nested JavaScript template literals, C# interpolated strings and Swift raw
  strings (`#"..."#`) do not count as code.
- Every other top-level form is rejected. The list is:
  - a second class, interface, record, struct, object, protocol, trait or
    `mixin`, annotated or not;
  - a function or method outside the class: `function`, `def`,
    `async def`, `fun` (including extension functions), `fn`, `func`, and a
    Dart or Swift top-level function;
  - an interface, type alias or enum outside the class: `interface`,
    `type`, `typealias`, `typedef`, `enum`, a Java `@interface`, a C#
    `delegate`, a Dart or Swift `extension`, and a PHP `trait`;
  - a constant or variable: `const`, `let`, `var`, `val`, `static`, a Python
    or PHP assignment;
  - a statement or expression: a call, `console.log(...)`,
    `if __name__ == "__main__":`, an `if TYPE_CHECKING:` block that holds
    anything other than imports, PHP `require`, a C# top-level statement, a
    string that is not the first statement, a second directive or docstring,
    and text outside `<?php ... ?>`;
  - code that runs when the file loads: a JavaScript or TypeScript
    `static { ... }` block (move its code into a method), a class header
    that computes its base class, such as `extends mixin(Base)` or a Python
    `class A(make_base()):`, and a decorator argument in JavaScript,
    TypeScript or Python that calls, constructs, assigns or computes
    anything, such as `@Cacheable(options())`, `@Service(new Options)` or a
    template literal with `${...}`; this covers the class's decorators and
    its members' decorators;
  - a re-export or module form: `export { ... }`, `export default Name`,
    `export * from`, `export default <expression>`,
    `import x = require(...)`, a TypeScript `namespace`, `declare module`, a
    second or nested C# namespace, code outside a C# namespace block, a C#
    preprocessor line other than `#nullable` (such as `#if`), a Dart `export`
    or `part`, a Rust `mod`, `macro_rules!`, `extern crate` or `impl` for
    another type;
  - an annotation or modifier with no class after it.
- An annotation starts only where the language allows one. In C#, `[`
  starts an attribute only at the start of a line or after `]`, `;`, `{` or
  `}`, so an index such as `new byte[size]` or `row[id]` is not an
  attribute. In Python, `@` starts a decorator only at the start of a line,
  so the matrix operator in `a @ b` is not one. In Kotlin, `this@Label`,
  `super@Label`, `return@label`, `break@label` and `continue@label` are
  labels. A plain helper that only indexes arrays or multiplies matrices
  is not an annotated provider. In `server/shared/`, it follows the plain
  shared-class composition rules below.
- A behavior-only class carries behavior annotations but no role annotation,
  for example a `@Validated` input, `@ConfigurationProperties` settings, a
  class with `@Value` fields, or a `@CacheConfig` class. It follows the
  one-class rule. The compiler discovers it in the realm's role folders, in
  the entry's language, and composes it into the same unit as the code that
  names it. It does not become an injectable provider. Place it in
  `delegates/`, the folder for the objects that roles use. A Tac service in
  `services/` may carry behavior annotations without `@Service`. (Amended by
  ADR 0091: a concrete class in a role folder now needs its role marker, so
  a Tac service needs `@Service`, and settings and validation data live in
  `models/` as ADR 0092 describes.) Where
  ADR 0021 and ADR 0022 say "the same source file" or "source-local", they
  now mean this composed unit. A behavior-only class outside the role
  folders, for example in `shared/`, is not discovered. There is no
  placement check for these classes. They have no role, and one class may
  carry several behavior families, so the compiler cannot read their
  purpose from their annotations; a check would have to guess. The
  one-class rule and same-language discovery still limit what is composed.
- A test unit file holds only its `@TachyonTest` class. Its
  `@TestConfiguration` class moves to its own file in the same `tests/`
  folder, named after the class (for example `ShellTestConfiguration.js`).
  The unit names it in `@TachyonTest({"configuration": "..."})` as today,
  and the compiler composes it into the unit, so it still never reaches a
  browser bundle.
- A file that breaks the rule fails with TY1310. The message names what else
  the file declares and where it belongs: another annotated class in its own
  file in its role folder, a type in `models/`, and plain code in `shared/`.
  A second annotated class is named as another annotated class that belongs
  in its own file. An annotation with nothing after it is named as an
  annotation or modifier with no class after it.
- The scaffold, the website, the documentation, the examples and the test
  fixtures follow the rule.

## Consequences

- A reader who opens an annotated file sees one class and what it imports.
- This is a breaking change. Projects with helper code in annotated files
  stop building until the code moves. The diagnostic names each item and its
  new place.
- Discovery now composes every same-language file in a role folder that
  carries a framework annotation, including behavior-only classes, not only
  files with a role annotation.
- Every test unit gains a configuration file next to it.
- Handler files and middleware lose their helper functions and classes.
  Helpers become service methods, which the handler injects, or plain code
  in `shared/`.
- Root middleware that declared a `Handler` class with HTTP-method methods
  stops building. It becomes a `Middleware` class with `before(request)` and
  `after(request)`. Before this change, the runtime called middleware by the
  request's HTTP method, so a middleware without `OPTIONS` or `POST`
  answered a CORS preflight or a form post with 405.
- Programs that Yon runs through `relay()` or a command gateway are not
  Tachyon source files, so the rule does not apply to them.

### Plain shared classes (added 2026-09-30)

`server/shared/` files also hold one plain class and its imports, without
Tachyon annotations or file-level variables. The class and file need no role
suffix or matching name. JavaScript and TypeScript use ordinary imports;
provider imports into this folder are rebased to the captured entry. Go files
share package main. Other Yon languages compose same-language shared classes
before providers and the controller, with F# declarations ordered by their
type references. Discovery is bounded and refuses symlinks, duplicate class
names and definition-time dependency cycles. Shared classes register no
providers or routes.
