# ADR 0046: Tachyon Annotations Are Global

- Status: Accepted
- Date: 2026-09-25
- Amends: [ADR 0022](0022-shared-behavior-adapters.md) (the named imports from
  `tachyon:behaviors`) and [ADR 0024](0024-strict-annotation-modules.md) (the
  compiler-owned import that included modules could keep)
- Preserves: every annotation name and its JSON arguments, the markers-only
  compile-time model, and the rule that Tachyon installs no runtime decorators
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them
- Amended by: [ADR 0053](0053-composed-classes-have-generated-typescript-globals.md): `ty` also generates global declarations for the application's composed JavaScript and TypeScript classes, one file per realm in `.tachyon/types/`

## Context

Tachyon finds its annotations by text in every language. In Python, PHP, Java,
Kotlin, C#, Rust, Swift, Dart and Go, an author writes a marker such as
`@Service` or `#[Service]` with no import. The compiler blanks the marker before
the language compiler sees the file.

JavaScript and TypeScript were the exception. ADR 0022 asked for a named import
from the compiler-owned `tachyon:behaviors` module, so that TypeScript could see
the types. The compiler validated that import, erased it, and merged it across
role files. The import made the two languages look different from the others,
and it was one more line to get wrong.

## Decision

- Tachyon annotations, lifecycle names (`PostConstruct`, `PreDestroy`,
  `EventListener`, `Order`, `Async`), `Behaviors` and the framework types
  (`Behavior*`, `Tac*`, the adapter and event types) are global in every
  language. A source file uses them without an import.
- In JavaScript and TypeScript, an import from `tachyon:behaviors` is refused.
  Every form is refused: named, type-only, default, namespace, side-effect,
  dynamic and re-export. The error is `TY1308`, and its message names the file
  and the line and says: "Tachyon annotations and types are global; delete
  this import". This is a breaking change.
- Tachyon still installs no global runtime decorators. The compiler erases the
  markers at compile time, as before. `Behaviors` is the generated binding that
  the compiler adds to each lowered module; an author still cannot declare it
  or import it from another module.
- The types are ambient declarations only (`declare global` in
  `tachyon-behaviors.d.ts`). They describe the compile-time markers for
  editors and `tsc`; they do not exist at run time. There is no
  `tachyon:behaviors` module declaration, so editors flag the old import.
- No editor stubs are added for the other languages.

## Consequences

- JavaScript and TypeScript sources read like the other languages: the marker
  and the class, with no framework import line.
- The compiler drops the code that validated, erased and merged the import.
- A module that uses only `Behaviors`, with no marker, is lowered too, so the
  binding exists at run time.
- Existing JavaScript and TypeScript projects must delete their
  `tachyon:behaviors` imports. The compiler names each one.
- The global names share the global scope with the DOM, Bun and Node types.
  `EventListener` is a DOM interface and a Tachyon function; they live in
  different declaration spaces and do not clash.
