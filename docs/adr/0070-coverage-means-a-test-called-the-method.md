# ADR 0070: Coverage Means a Test Called the Method

- Status: Accepted
- Date: 2026-09-27
- Amends: the method coverage rule of the test units (`TY2022`, `TY2023`),
  introduced with compiled test units; preserves
  [ADR 0063](0063-services-and-controllers-are-cohesive.md) (services and
  controllers are cohesive) and
  [ADR 0064](0064-code-is-not-duplicated.md) (utility classes)

## Context

Every public method of an annotated class, and every public static method of a
utility class, needs a test. `ty preview` refused to start, and
`ty test --coverage` failed, when no `test*` case covered such a method. But
"covered" meant only that a case in a unit that names the class also wrote the
name of the method. A test could name a method in a string, or call it only on
a Fake, and the method still counted as tested. The rule promised more than it
checked.

## Decision

A method is covered only when a test case called it while `ty test` ran.

1. **Recording lives in the class, only in test builds.** When the compiler
   assembles a test unit (an entry in `client/tests` or `server/tests`), it
   adds one statement at the start of each counted public method of every
   composed role file and utility class. The statement records
   `Type.method`. Production builds never assemble a test entry, so they have
   no recording statement and pay nothing. The same public-method reader
   decides what is counted and what is recorded, so the two cannot disagree.
2. **One recorder per language runtime.** JavaScript and TypeScript use
   `Behaviors.called`; the polyglot runtimes (Python, Ruby, PHP, Java, Kotlin,
   C#, Rust, Go, Dart, Swift) have a small `BehaviorCalls` recorder. The test
   driver clears it before each case and reports the recorded names with the
   case (`called`). This covers all eleven Yon and nine Tac test languages.
3. **Real instances count, Fakes do not.** Because the statement is inside the
   real method body, any real instance counts: one made by `construct`, by
   `new`, or by a real `Needs` dependency. A Fake is a different object, so a
   call on it never reaches the real body.
4. **The record.** `ty test` merges the reports, keeps the first case that
   called each method per realm, and writes
   `.tachyon/coverage/calls.json` (`TY2024` when it cannot). With `--coverage`
   it prints every class with its called methods and the first case, and it
   fails with `TY2022` when a method was not called. The record also states
   whether all executed cases passed. Before any new test run, the previous
   record is marked unsuccessful; a write failure stops the run. This keeps
   an interrupted or failed run from leaving an older successful record valid.
5. **Preview.** `ty preview` reads the record. When the record is missing or
   older than a role file or a test, or does not record a successful run, it
   runs the test units once and writes a new record. Records written before
   the run-result field was added are read but must be refreshed. It refuses
   to start with `TY2023` and
   `no test case called X.y`. `TAC_ALLOW_UNTESTED=1`, the exempt method names,
   and `ty start` (never checked) are unchanged. The compatibility `ty serve`
   command does not run this gate either. A failed automatic test case stops
   preview with `TY2020`, even if that case called every required method.
   Successful automatic runs are quiet: the first stdout line remains the
   readiness announcement. Explicit `ty test` keeps its case output.
6. **Fallback, clearly labelled.** When a language has test units in a realm
   but none of them could run on this machine (for example Swift units where
   swiftc is missing), `ty` cannot record their calls. For that language only
   it uses the old name check, and the report marks the class
   `[name check]`. This is a coverage-report fallback, not startup approval.
   A failed run stays unsuccessful, and required runtime checks still apply.

Expression-bodied methods keep their value: Kotlin `= expr` becomes
`= BehaviorCalls.record(...) behaviorThen expr`, a Dart arrow becomes a block
that records and then returns, and a Swift single-expression body gets an
explicit `return`.

## Consequences

- A test that names a method but does not call it now fails coverage, and so
  does a method that only a Fake answers. The website, the scaffolds, the
  tutorial and the controller fixtures call every counted method.
- `ty preview` may run the tests before it starts, after sources changed. The
  website's 44 cases take about ten seconds with warm caches.
- A method called only by a background task after its case ended may be
  reported with the next case. The first case is a hint for the reader, not
  part of the rule.

## Amendment: Imported Tac script utilities (2026-10-03)

A Tac JavaScript or TypeScript utility in `client/shared/utils` that another
client script imports by an absolute or relative path, or that uses
`import.meta`, loads as its own module in a test unit. The test build does
not add the recording line to such a module, so its calls cannot be recorded.
The coverage report leaves these utilities out (`ImportedUtilities` in
`method_coverage`) instead of reporting them as untested. Utilities that are
composed into the program are still recorded and required. Recording calls of
imported utilities is a separate follow-up.

## Amendment: Recorded imported Tac script utilities (2026-10-04)

This amendment replaces the exclusion of the 2026-10-03 amendment. A
JavaScript or TypeScript test unit now runs with recorded copies of every
script in `client/shared/utils`. Before the unit starts, `ty test` writes each
utility to the unit's staging folder with the recording line in each public
static method, and with a first line that sends the calls to the unit's
`Behaviors` through one global recorder. The unit's preload resolves an
absolute path such as `/shared/utils/PriceUtils.js` to that copy first; the
unit file sits at the same relative place as its source, so a relative import
such as `../shared/utils/PriceUtils.js` reaches the copy too. Node, Bun and
Deno (through its import map) use the same copies. So the coverage report
lists every client utility again, and a method of an imported utility that no
case called is missing like any other. Utilities in the other Tac languages
were never affected: their units compose `client/shared/utils` into one
program, where the test build already adds the recording line.
