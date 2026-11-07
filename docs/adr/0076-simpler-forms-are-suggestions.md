# ADR 0076: Simpler Forms Are Suggestions

- Status: Accepted
- Date: 2026-09-28
- Preserves: [ADR 0063](0063-services-and-controllers-are-cohesive.md) (a
  public method only coordinates) and
  [ADR 0069](0069-design-rules-are-build-errors.md) (design rules are build
  errors)

## Context

Some code does its job but makes the reader stop and think:
`if (a > b) { return true; } return false;` only returns the comparison,
`items.filter(p).length > 0` only asks whether one item matches, and an
`else` after a `return` only adds a level of indentation. The shared
engineering rules ask for the simpler form, but a reader who learns English
as a second language gains nothing from a line that is merely shorter. A
dense line with several parts is harder, not simpler.

## Decision

Tachyon knows a curated list of patterns (TY1340 to TY1355, one code each).
Each rewrite reads more plainly and does exactly the same thing. The list:
boolean return (1340), comparison with a boolean (1341), true/false choice
(1342), any match instead of a filter (1343), contains instead of a
position (1344), `else` after an exit (1345), double negation (1346),
negated equality (1347), a bare `return` at the end (1348), an `if` that
holds only an `if` (1349), compound assignment (1350), read with a default
(1351), a loop that only joins or adds up (1352), is empty (1353), a loop
over items instead of indexes (1354) and a boolean assignment (1355).

- **Suggestions, never errors.** `ty build`, `ty preview` and `ty test` print
  `suggestion[TY13xx]` with the file, the line and a `help:` line that shows
  the rewritten code. The command goes on.
- **One fix.** `ty migrate fix` lists every rewrite; `--write` applies them.
  Each rewrite is tested, idempotent, and skipped when it overlaps another
  change.
- **Same behaviour or no suggestion.** A pattern is suggested only where the
  simpler form behaves the same. JavaScript, TypeScript, PHP, Python and
  Ruby conditions need a comparison, a `!` or another true-or-false value
  before `return cond` is suggested; `x == true` is suggested only in Java
  and Rust (a Go value of type `any` compares with `true` but is not a
  condition by itself); `+=` in Python and Kotlin only for a variable that started as
  a number or text; no edit crosses a comment, a multi-line string, or a
  block-scoped declaration.
- **Per realm.** `tac.simplify.suggest` in `client/manifest.json` and
  `simplify.suggest` in `server/application.json` (both `true` by default)
  hide the suggestions of one side.
- **Engine.** The scan reuses the tokenizer and word list of the cohesion
  guard, joins operators into pieces, and runs in linear time. Files over
  256 KB, `*.min.*` files and `assets` folders are skipped.

## Decided against

- Python `len(items) == 0` to `not items`: it relies on the truthiness of
  containers, and it differs for NumPy arrays.
- Ruby `if !x` to `unless x`: `unless` is a harder word, and `unless ... else`
  reads badly.
- Python and Ruby `sum` for a loop that adds numbers: both add decimal
  numbers with compensated summation, so the result can differ from the loop.
- C# `IndexOf` to `Contains` (culture-sensitive comparison), Rust iterator
  `find` to `any` (different closure argument types), reduce and fold
  chains, nested choices and comprehensions.

## Consequences

The build output shows more lines, but none of them stops a build. Examples
on the website and in the templates are written in the simpler forms, so a
new project starts with no suggestions.

## Receiver suggestion amendment — 2026-10-01

TY1362 extends this family without renumbering its original sixteen codes.
Java, Kotlin, C#, Groovy, Swift and Dart can access class members by their
bare names. The suggestion removes an explicit receiver only inside an owned
method, for a declared member, when no unqualified occurrence of the same name
could indicate a parameter or local binding. Nested blocks are conservatively
excluded, preserving Swift captured closures and Groovy/Kotlin receiver scopes.
Qualified outer receivers, Dart initializing formals, strings and comments stay
unchanged. Required receivers in other languages are outside this rule.
Receiver scans additionally skip files over 16,384 tokens or 256 explicit member
accesses, bounding conservative scope checks. Disabled realms do not run the
receiver scan. Edits share the existing overlap filter before migration writes.

This suggestion has its own opt-in switches: `tac.implicitmembers.suggest`
in the client manifest and `implicitmembers.suggest` in the server application
configuration. Both default to false. Existing simpler-form settings and codes
keep their behavior. A migration applies receiver edits only when the matching
realm opted in.

Groovy remains a Yon language. Its source and call-hook coverage registration
must include `.groovy` services, with the existing Java syntax reader and JVM
runtime. This amendment does not introduce Tac Groovy support. F# private values
may use `let`; F# members retain the self identifier required by that language.

## Receiver elision feasibility amendment — 2026-10-16

This section is design only. It changes no code and adds no suggestion code.
It records, for every Tac and Yon language, whether a member can be written by
its bare name instead of through a receiver, and whether TY1362 could suggest
that rewrite. The answer depends on two things: the language's own name
lookup, and how Tachyon reads and lowers the authored class.

Two facts hold for every language:

- **The compiler does not depend on the authored receiver.** The web host and
  the Yon runtimes read service and controller state by member name across
  their boundary, and the code that the compiler generates writes its own
  receivers (for example `this.{field} = {field}` in
  `behavior/mod.rs`, `$this->{name}` in `behavior/polyglot/php.rs`,
  `self.{name}` in `behavior/polyglot/swift.rs`, and the `self.` call style for
  Rust in `behavior/call_style.rs`). So elision is only a question of the
  source language. It never needs a lowering change.
- **A suggestion needs a reader.** TY1362 uses the cohesion tokenizer and its
  method finder. That reader knows `js`, `ts`, `java`, `cs`, `dart`, `kt`,
  `swift`, `php`, `rs`, `go`, `py` and `rb`, and Groovy through the Java
  syntax (`cohesion/spelling.rs`, `groovy_source.rs`). It does not read Lua,
  F# or C++.

| Language | Realms | Receiver in authored code | Bare member name means | Elision | TY1362 |
| --- | --- | --- | --- | --- | --- |
| Java | Tac, Yon | `this.x` optional | the field, unless a local or parameter has the name | Possible | Done |
| Kotlin | Tac, Yon | `this.x` optional | the property; nested lambdas can have another receiver | Possible outside nested blocks | Done |
| C# | Tac, Yon | `this.X` optional; the scaffold already writes `Message` | the field or property | Possible | Done |
| Groovy | Yon | `this.x` optional | the property; closures can change the delegate | Possible outside nested blocks | Done |
| Swift | Tac | `self.x` optional; the scaffold already writes `message` | the property; escaping closures need `self` | Possible outside nested blocks | Done |
| Dart | Tac, Yon | `this.x` optional, except `this.x` initializing formals | the field | Possible outside initializing formals | Done |
| C++ | Tac | `this->x` optional; the scaffold already writes `message` | the member, unless a local or parameter has the name | Possible | Not possible yet: the reader does not know `.hpp`, and the receiver is `this->`, not `this.` |
| JavaScript | Tac, Yon | `this.x` required | a module, global or closure binding, never the field | Not possible | Out of scope |
| TypeScript | Tac, Yon | `this.x` required | the same as JavaScript | Not possible | Out of scope |
| Python | Tac, Yon | `self.x` required (`self` is an ordinary parameter) | a local, then a module name; an assignment makes a new local | Not possible | Out of scope |
| Ruby | Tac, Yon | `@x` for instance variables; `attr_reader` makes a method | a reader can be called as `x`, but `x = value` makes a local and never calls a writer | Not possible as a receiver rewrite; reading through `attr_reader` is already a bare call, and `@x` is not a receiver | Out of scope |
| Lua | Tac, Yon | `self.x` required (`self` comes from `function T:m()`) | a local, then a global; the scaffold class table itself is a global | Not possible | Out of scope; the Lua shadow writer also writes `self.` fields for the type checker (`lua_source/shadow_writer.rs`) |
| PHP | Yon | `$this->x` required | a local variable `$x` | Not possible | Out of scope |
| Go | Tac, Yon | the named receiver, such as `greeting.Message` | a package-level name | Not possible | Out of scope |
| Rust | Tac, Yon | `self.x` required | a local, then an item in scope | Not possible | Out of scope |
| F# | Tac, Yon | `member this.M()` needs a self identifier, and `this.X` is required for members | a `let` value of the type, or an outer binding | Not possible for members; private `let` values are already bare | Out of scope; F# behavior lowering also calls the hidden method as `this.<name>` (`fsharp_source/fsharp_behaviors.rs`) |

Result: the six languages that TY1362 covers are the only ones where the
language allows a bare member name and Tachyon already has a reader. C++ is the
one remaining language where elision is legal. A C++ rule would need the C++
tokenizer from `tac_cpp` in the suggestion scan and a `this->` receiver form,
with the same owned-method, declared-member and no-shadowing checks. Because
the C++ scaffold already writes bare members, that rule would help only code
written by hand, so it stays a later option and is not planned here. In the
other nine languages a bare name does not mean the member, so no suggestion
and no compiler rewrite can offer it without changing behavior.
