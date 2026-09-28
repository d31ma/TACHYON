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
