# ADR 0080: A Test Case Asserts and Holds No Logic

- Status: Accepted
- Date: 2026-09-28
- Extends: [ADR 0069](0069-design-rules-are-build-errors.md) (design rules
  are build errors) with a seventh rule
- Preserves: [ADR 0070](0070-coverage-means-a-test-called-the-method.md)
  (a method is covered only when a test case called it) and the naming
  rule of test units (a case is a method whose name begins with `test`)

## Context

Most test cases in Tachyon projects checked a result with a branch:
`if (total !== 8) throw Error('expected 8')`, `raise ... unless ...`, or
`return Err(...)`. A test with a branch or a loop is code that can be wrong
itself, and it does not say what it expected when it fails. Tests that
repeat one case for several inputs copied the case or looped over the
inputs, so one failing input hid the others.

Developers who come from Spring and JUnit expect `assertEquals`,
`assertTrue`, `assertThrows` and `@ParameterizedTest`.

## Decision

### 1. One assertion helper, named like JUnit 5

The driver of every test unit gives the unit a class `Assert`:

| Language | Equal | True | Throws |
| --- | --- | --- | --- |
| JavaScript, TypeScript | `Assert.equals(expected, actual)` | `Assert.isTrue(condition)` | `Assert.throws(action)` |
| Python | `Assert.equals(expected, actual)` | `Assert.is_true(condition)` | `Assert.throws(action)` |
| Rust | `Assert::equals(expected, actual)` | `Assert::is_true(condition)` | `Assert::throws(action)` |
| Java, Groovy | `Assert.equals(expected, actual)` | `Assert.isTrue(condition)` | `Assert.throwsError(action)` |
| Kotlin | `Assert.equals(expected, actual)` | `Assert.isTrue(condition)` | `Assert.throws { action }` |
| C#, F# | `Assert.equals(expected, actual)` | `Assert.isTrue(condition)` | `Assert.throws(action)` |

Each check takes an optional last message, like JUnit. Rust has no optional
arguments, so it has `equals_with_message`, `is_true_with_message` and
`throws_with_message`. A failed check fails the case with JUnit's text:
`the total ==> expected: <3> but was: <2>`. `equals` compares values, not
references: arrays and objects are equal when their items are equal.
`throws` returns the error, so the case can check its message. When the
action is asynchronous, `throws` returns a promise (JavaScript) or an
awaitable (Python) that the case awaits. In Rust, `throws` passes when the
closure returns an `Err` or panics. `throws` is a reserved word in Java and
Groovy, so there the method is `throwsError`. In Java, Kotlin and Groovy,
`equals` treats two whole numbers of different sizes (`int` and `long`) as
equal when their values are equal, so `Assert.equals(1200, total)` passes for
a `long` total; the helper throws `AssertionError`, as JUnit does. C# and
F# compare whole numbers the same way and compare two sequences item by
item. In C#, `throws` also takes an `async` lambda, and in F# an action that
returns a task; both wait for the task. The C# helper is added to a lowered
C# test unit (Yon and Tac alike), not to the `yon.cs` prelude, so that other
handlers never see a class named `Assert`; F# test units get an F# `Assert`
type next to the test class.

The assert of the language counts too, when it is always on: the Python
`assert` statement and the Rust macros `assert!`, `assert_eq!` and
`assert_ne!`, the Java and Groovy `assert` statement and the Kotlin functions
`check` and `require`, the PHP `assert` function, the Dart `assert` statement
and the Swift `precondition` function. Java needs `-ea` to run `assert`, so Tachyon starts a
Java test unit (a handler under `server/tests`) with `java -ea`; controllers
and workers keep assertions off. Groovy runs its `assert` always. The Kotlin
`assert` function does not count, because it runs only with `-ea`. A Rust case that panics now fails as one case; before, the
panic stopped the whole unit. `console.assert` does not count, because it
only logs. `debug_assert!` does not count, because a release build removes
it. For the same reason the C# `Debug.Assert` and the F# `assert` do not
count: Tachyon builds .NET test units in Release mode. .NET has no assert
that is always on and throws. PHP turns `assert` off in production
(`zend.assertions=-1`), so Tachyon starts a PHP test unit with
`zend.assertions=1`; Dart runs `assert` only with `--enable-asserts`, which
Tachyon passes to the Tac `dart run` and to the Yon AOT compile of a test
unit. A failed Swift `precondition` stops the whole process, so the unit
reports one failure instead of one per case; `Assert` is the better check in
Swift. The Swift `assert` does not count, because optimized builds leave it
out. Go and Ruby have no assert of their own.

Go spells the helper with capitals (`Assert.Equals`, `Assert.IsTrue`,
`Assert.Throws`), because only capitalized Go names are exported; Ruby uses
`is_true` and a block for `throws`; PHP uses `Assert::`; Dart awaits
`Assert.throws`; Swift writes `try` before a check, because a failed check
throws. `throws` is not reserved in these five languages (Swift allows it
after a dot), so only Java and Groovy use `throwsError`. A failed Go check
panics, and the driver recovers the panic into one failed case, so a Go case
may return nothing; a case that returns `error` still works.

### 2. Data-driven cases, like JUnit's @ValueSource

`ValueSource` on a test case lists its inputs. The driver runs the case once
for each input and passes the input as the one argument. The cases are named
`testKeepsPositiveInputs[1]`, `testKeepsPositiveInputs[2]` and so on, so one
failing input does not hide the others.

```ts
@ValueSource([1, 2, 3])
async testKeepsPositiveInputs(value: number): Promise<void> {
  Assert.isTrue(value > 0)
}
```

As in JUnit, the list holds literal values (at most 256). In Rust, Java,
Kotlin, Groovy, C#, F#, Go, Dart and Swift it holds numbers, strings and
booleans, because the driver writes each input into a typed call (F# writes the JSON list with
commas: `[<ValueSource([0, 5, 1000])>]`) (a `$` in a string is written as
`\u0024`, which all three JVM languages read as a plain `$`). The annotation belongs on a `test` method of a `Test` class.
Methods are found by name, so no `@ParameterizedTest` is needed.

### 3. The rule (TY1339)

The build refuses a test case (a `test` method of a `Test` class in
`client/tests` or `server/tests`) that has no assertion, or that holds a
branch or a loop: `if`, `else`, `for`, `while`, `do`, `switch`, `case`,
`match`, `loop`, `elif`, `when` (Kotlin, C#, F#, Ruby), `foreach` (C#, PHP),
`elseif` (PHP), `elsif`, `unless` and `until` (Ruby), `guard` and `repeat`
(Swift), `select` and `goto` (Go) and the choice operator `? :`. A Go case is a
method whose name begins with `Test`. F# is read through its C# view, so `match` and
`function` count as `switch` there; `ty build` names the F# word. The Python form
`a if condition else b` is a branch too. `&&` and `||` inside a check are
allowed. A helper method of the unit (a name that does not begin with
`test`) may hold logic.

The rule is the seventh design rule of ADR 0069: same module, same
diagnostics, same `ty migrate check` findings, and the same switch per
realm: `tests.check` in `server/application.json` and `tac.tests.check` in
`client/manifest.json`. It is on by default; a realm turns it off with
`"check": false`.

### Slices

The first slice covers JavaScript, TypeScript, Python and Rust (helper,
`ValueSource` and rule); the second rewrites their test units. The third
adds Java, Kotlin and Groovy and rewrites their units. The fourth adds C#
and F# and rewrites their units. The fifth adds Go, Ruby, PHP, Dart and
Swift and rewrites their units. The sixth slice turns the rule on by default.

## Consequences

- A failing check says what it expected and what it got.
- One case per input: a report shows every failing input.
- There is no automatic rewrite in `ty migrate fix`; the forms differ too
  much between languages. `ty migrate check` lists every case to change.
- `Assert` is a name that a test unit cannot use for its own class.

## Amendment (2026-10-19): TestSettings reach the context the same way in every language

A run test in every test language (PR3 of the platform plan) found that
`TestSettings` stopped the unit before its first case in the JavaScript,
TypeScript, Python, PHP, Dart, Go, Rust, Ruby and Swift drivers. Those
drivers passed each path as written (`yon.proof.mode`) and Go, Rust, Dart
and Swift also passed each value as JSON text (`"on"` with its quotes).
The behavior runtimes only accept environment names and plain text values,
so the context refused the properties or a `WhenSetting` never matched.

Every driver now passes the same form that Java, Kotlin, C# and F# already
used: the path as its environment name (`yon.proof.mode` is
`YON_PROOF_MODE`) and the value as text (`5` is `5`, `true` is `true`).
Lua and C++ read the properties in their own runtimes and were already
right.

`tests/it/test_marker_proofs.rs` keeps this true. In each Yon and Tac test
language, a unit with `UseProfiles`, `TestSettings`, `FreshStart` and a
`ValueSource` case passes against a TestSetup gated by `WhenProfile` and
`WhenSetting`, and each case sees a context no earlier case saw. The same
unit without `UseProfiles`, or without `TestSettings`, stops with an
inactive configuration. A second test runs a Tac service with
`Description` and a `Cached` method whose `CacheKey` leaves a parameter
out of the key.
