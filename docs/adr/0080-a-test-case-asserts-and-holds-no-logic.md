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
a `long` total; the helper throws `AssertionError`, as JUnit does.

The assert of the language counts too, when it is always on: the Python
`assert` statement and the Rust macros `assert!`, `assert_eq!` and
`assert_ne!`, the Java and Groovy `assert` statement and the Kotlin functions
`check` and `require`. Java needs `-ea` to run `assert`, so Tachyon starts a
Java test unit (a handler under `server/tests`) with `java -ea`; controllers
and workers keep assertions off. Groovy runs its `assert` always. The Kotlin
`assert` function does not count, because it runs only with `-ea`. A Rust case that panics now fails as one case; before, the
panic stopped the whole unit. `console.assert` does not count, because it
only logs. `debug_assert!` does not count, because a release build removes
it.

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
Kotlin and Groovy it holds numbers, strings and booleans, because the driver
writes each input into a typed call (a `$` in a string is written as
`\u0024`, which all three JVM languages read as a plain `$`). The annotation belongs on a `test` method of a `Test` class.
Methods are found by name, so no `@ParameterizedTest` is needed.

### 3. The rule (TY1339)

The build refuses a test case (a `test` method of a `Test` class in
`client/tests` or `server/tests`) that has no assertion, or that holds a
branch or a loop: `if`, `else`, `for`, `while`, `do`, `switch`, `case`,
`match`, `loop`, `elif`, `when` (Kotlin) and the choice operator `? :`. The Python form
`a if condition else b` is a branch too. `&&` and `||` inside a check are
allowed. A helper method of the unit (a name that does not begin with
`test`) may hold logic.

The rule is the seventh design rule of ADR 0069: same module, same
diagnostics, same `ty migrate check` findings, and the same switch per
realm: `tests.check` in `server/application.json` and `tac.tests.check` in
`client/manifest.json`. It is **off by default** until every test unit of
the website, the fixtures and the `ty init` templates is rewritten.

### Slices

The first slice covers JavaScript, TypeScript, Python and Rust (helper,
`ValueSource` and rule); the second rewrites their test units. The third
adds Java, Kotlin and Groovy and rewrites their units. Later slices add the
other test languages; the last slice turns the rule on.

## Consequences

- A failing check says what it expected and what it got.
- One case per input: a report shows every failing input.
- There is no automatic rewrite in `ty migrate fix`; the forms differ too
  much between languages. `ty migrate check` lists every case to change.
- `Assert` is a name that a test unit cannot use for its own class.
