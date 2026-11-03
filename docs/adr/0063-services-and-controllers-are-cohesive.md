# ADR 0063: Services and Controllers Are Cohesive

- Status: Accepted
- Date: 2026-09-27
- Preserves: [ADR 0044](0044-an-annotated-file-holds-one-class.md) (an
  annotated file holds one class) and [ADR 0056](0056-every-annotation-checks-its-target.md) (the build
  checks the rules, not the reviewer)

## Context

Services and controllers grew long public methods that mixed coordination
with decisions: a route method read a value, checked it, looped over a list and
caught an error in one body. A reader had to follow every branch to learn what
the entry point does, and a test had to cover every branch through the public
method. The same rule already applies to Tachyon's own code (the shared
engineering instructions): a public function only coordinates, and each
private helper holds at most one decision.

## Decision

The build checks every class in `client/services`, `server/services` and
`server/controllers` (error handlers included) in every Tac and Yon language.

1. **A public method only coordinates.** It calls injected services and
   private methods, stores or returns their results, and builds models and
   values. It holds no construct from the table below, also not inside a
   lambda that it writes.
2. **A private method holds at most one decision or one loop.** An
   `else if` / `elif` / `elsif` / `elseif` chain is one decision, and an `&&`
   or `||` inside the condition of an `if`, `while`, `unless`, `until` or
   `guard` is part of that decision. `try`, `finally`, `begin` and `ensure`
   do not count in a private method; `catch`, `except` and `rescue` do.
3. **A private method that decides does not call another private method that
   decides.** A loop is not a decision for this rule. Calls through `this`,
   `self`, `$this`, `Self::`, the Go receiver or a bare name count.

A break is refused with `TY1314`. The message names the method and the
construct, and the help shows the private spelling of the language. `ty
migrate check` lists every place as a `cohesion.method` finding with its line.

| Language | Constructs | Private method |
|---|---|---|
| JavaScript, TypeScript | `if`, `switch`, `for`, `while`, `try`/`catch`/`finally`, `&&`, `\|\|`, `??`, `? :` | `#name`; TypeScript also `private` |
| Java | `if`, `switch`, `for`, `while`, `try`/`catch`/`finally`, `&&`, `\|\|`, `? :` | `private` |
| C# | Java's list, `foreach`, `??`, query syntax `from x in` | `private` |
| Kotlin | `if`, `when`, `for`, `while`, `try`/`catch`/`finally`, `&&`, `\|\|`, `?:` | `private` |
| Swift | `if`, `guard`, `switch`, `for`, `while`, `catch`, `&&`, `\|\|`, `??`, `? :` | `private`, `fileprivate` |
| Dart | `if`, `switch`, `for`, `while`, `try`/`catch`/`finally`, `&&`, `\|\|`, `??`, `? :` (also collection `if`/`for`) | a name starting with `_` |
| PHP | `if`, `elseif`, `switch`, `match`, `for`, `foreach`, `while`, `try`/`catch`/`finally`, `&&`, `\|\|`, `??`, `?:`, `? :`, `and`, `or`, `xor` | `private` |
| Python | `if`, `elif`, `for` (comprehensions too), `while`, `match`, `try`/`except`/`finally`, `and`, `or` | a name starting with `_` (dunders stay public) |
| Ruby | `if`, `unless`, `elsif`, `case`, `while`, `until`, `for`, `loop`, `begin`/`rescue`/`ensure`, `&&`, `\|\|`, `and`, `or`, `? :` | below `private`, `private def`, `private :name` |
| Go | `if`, `switch`, `select`, `for`, `&&`, `\|\|` | a lowercase name |
| Rust | `if`, `match`, `for`, `while`, `loop`, `&&`, `\|\|` | no `pub`; methods of `impl Trait for Type` and of a `pub trait` are public |

In every language a method whose name starts with `_` is private. An
iterator call (`map`, `filter`, `forEach`, `each`, `Select`, `for_each` and
their relatives) counts as a loop only when its lambda or block holds one of
the constructs; `items.map(item => item.id)` and a method reference are
fine. Optional chaining (`?.`) and Rust's `?` operator are allowed.

The scanner is token based. It reuses the one-class layout token reader
(`annotation_layout/single_class/tokens.rs`) and the lexical masks of each
language, so strings, comments, template literals, regular expressions and
interpolations never count. Per-language spellings live in one table,
`annotation_layout/single_class/cohesion/spelling.rs`.

One switch per realm turns the check off while a project migrates:
`tac.cohesion.check: false` in `client/manifest.json` for Tac services, and
`cohesion.check: false` in `server/application.json` (the `yon.cohesion.check`
path) for Yon services and controllers. `ty migrate check` ignores the switch, so it keeps listing the
places that still need a change.

## Consequences

- Entry points read as a short list of steps; each decision has a name.
- Tests can reach each decision through a small private method's caller
  instead of through a long public method.
- A ternary, `?:` or `??` default in a public method moves to a private
  method; so does error handling (or it moves to an `ErrorHandler`).
- The scanner reads tokens, not a full parse tree. It does not see a
  property getter (a Dart `get`, a C# `=>` property, a Swift computed
  property) as a method, and it reads a Ruby endless `def` as ending at the
  next `end`.
