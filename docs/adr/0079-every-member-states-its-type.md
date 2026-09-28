# ADR 0079: Every Member States Its Type

- Status: Accepted
- Date: 2026-09-28
- Extends: [ADR 0069](0069-design-rules-are-build-errors.md) (design rules
  are build errors) with a sixth rule
- Preserves: [ADR 0067](0067-a-class-file-is-named-after-its-class.md)
  (a JavaScript model is a JSDoc typedef) and the no-comments rule: a type
  comment is a comment that a tool reads

## Context

Tachyon reads types to check routes, models, events and injection, and the
language checkers (`tsc`, mypy, PHP, YARD) read them to find mistakes before
a request does. In a language whose syntax has no types (JavaScript, Ruby) a
member without a type comment is invisible to all of them. In a language
whose types are optional (Python, PHP, TypeScript without strict mode) an
untyped member is the same gap. The examples on the website mixed both: some
fields had `@type`, most parameters had nothing, and one website service
started with `// @ts-nocheck`, which switches the whole checker off.

## Decision

The build refuses a class member without a type (`TY1356`). The check is
the sixth design rule of ADR 0069: it runs in the same module, on the same
class files (every role folder of `client/` and `server/`, public and
private members, not test files), and it has the same switch,
`types.check` in `server/application.json` and `tac.types.check` in
`client/manifest.json`. It is **off by default** until the website, the
fixtures, the corpus and the `ty init` templates pass it.

### What each language writes

| Language | Parameter | Return value | Field |
| --- | --- | --- | --- |
| JavaScript | `@param {T} name` | `@returns {T}` | `/** @type {T} */` |
| Ruby | `# @param name [T]` | `# @return [T]` | `# @return [T]` above `attr_reader`, `attr_accessor`, `attr_writer` |
| Python | `name: T` | `-> T` | `name: T = value` |
| PHP | `T $name` | `): T` | `private T $name` |
| TypeScript | `name: T` (or a default value) | inferred | `name: T` (or a starting value) |

JavaScript and Ruby type comments sit in the comment block directly above
the member; annotation lines (`@Needs`, `#[Needs]`) may stand between them.
Groovy members already need a declared type (`TY1334`, ADR 0072); the typed
languages (Java, Kotlin, C#, Dart, Swift, Go, Rust, F#) need no rule.

### What needs no type

- A return type where the method gives back nothing: JavaScript without a
  `return value`, constructors (`constructor`, `__init__`, `__construct`,
  `initialize`). Python and PHP still write `-> None` and `: void`, because
  the native syntax is cheap and `ty migrate fix` adds it.
- A field whose starting value is a single number, string or boolean in
  JavaScript and Python (`count = 0`, `LIMIT = 3`): every checker reads the
  literal's type exactly, as TypeScript does. A PHP `const` is not a field.
- Local variables.

### Types must be correct

The rule checks that a type is written; the language's own checker checks
that it is right. JavaScript types are checked by `tsc` with `checkJs` and
`strict` (the `jsconfig.json` that `ty init` writes), Python hints by the
interpreter and mypy, PHP types by PHP. Ruby has no such checker in the
build, so the rule itself checks every name in a YARD type: it names a Ruby
core class, a class or model file of the project, a Tachyon class, a
qualified library class (`Net::HTTPGenericRequest`), or `Boolean`, `nil`,
`true`, `false`, `void` or `self`.

### JavaScript fields start with a value or are filled

Amendment (2026-09-28). `tsc` with `strict` also checks that every field
gets a value in the constructor (`strictPropertyInitialization`, `TS2564`).
TypeScript marks a field that Tachyon fills later with `!`
(`taxHelper!: TaxHelper`, `TY1331`); JavaScript has no such mark, so the
check refused every `@Needs` field. The `jsconfig.json` that `ty init`
writes for a realm with JavaScript code turns only this check off; a realm
that uses only TypeScript keeps the full `strict` check. The type rule
checks the fields instead: in a JavaScript class file, a field without a
starting value is refused with `TY1356` unless Tachyon fills it. Tachyon
fills a field marked `@Needs`, `@Setting` or `@Fake`, every field of a class
marked `@NeedsAll` or `@Needs(...)`, and the fields of an `@Entity`. The
message names the field; the help shows both fixes, a starting value or
`@Needs`.

### No escape hatches in class files

A class file may not switch the checker off: `// @ts-nocheck`,
`// @ts-ignore` and `// @ts-expect-error` in JavaScript and TypeScript, and
`# type: ignore`, `# mypy: ignore-errors` and `# pyright: ignore` in Python
are refused with the same code. The simplest safe choice is to refuse all of
them, also the one-line forms: `@ts-ignore` silently hides every error on
its line, and a reason line cannot be checked. Test files are not class
files, so a type test may still use `@ts-expect-error` to prove that a wrong
call does not compile.

### The fix

`ty migrate fix --write` adds the types it can know, where the project has
switched the rule on:

- the field of an injected class (a field without a starting value whose
  name is the class's instance name): `/** @type {TaxHelper} */`,
  `taxHelper!: TaxHelper`, `public TaxHelper $taxHelper`,
  `# @return [TaxHelper]`;
- `-> None` in Python and `: void` in PHP for a method that gives back
  nothing.

Every other missing type is listed as a manual step with the exact comment
or annotation to add.

## Consequences

- Every example shows the types a reader needs, in the form the language's
  checker reads.
- A project turns the rule on per realm, runs `ty migrate fix --write`, and
  types the listed members by hand.
- The rule reads types; it does not infer them. A wrong JavaScript type is
  found by `tsc`, not by the build.
