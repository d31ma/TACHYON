# Phase 6 Specification: Compatibility and Migration

This document defines the rules for Phase 6. It defines:

- how we measure the Rust implementation against the archived v26.30.04
  release;
- what a compatibility claim requires;
- what `ty migrate check` must report.

Phase 6 adds no feature for applications. It adds measurement. It also adds
the words we need to state a compatibility claim that a test can prove wrong.

## 1. The Oracle

The v26.30.04 executable is the reference for correct behavior (the oracle).
CI (continuous integration) downloads it and verifies its checksum. It is
never committed into this branch. A compatibility claim describes behavior
that can be observed from outside. It never describes internal structure.

## 2. What Is Compared

We do **not** compare build artifacts. The two implementations produce
different output on purpose:

- The legacy build produces a single-page shell with a client router, a
  service worker, and one chunk per page.
- The Rust build produces a bootstrap per route, a compiled render plan, and
  the Tac client runtime.

A byte comparison of these outputs would measure nothing.

We compare what a user or an assistive technology can observe:

1. **Route graph**: the set of published routes and their canonical order.
2. **Semantic DOM**: for every route, the rendered document reduced to element
   names, visible text, and the attributes that have meaning
   (`alt`, `aria-*`, `for`, `href`, `name`, `placeholder`, `role`, `src`,
   `type`, `value`). The comparison excludes scripts, styles, links, metadata,
   and every attribute that is specific to one implementation.
3. **HTTP behavior**: the status code served for each route.
4. **Diagnostics**: if one implementation rejects an input, the other must
   also reject it. Otherwise, the ledger must record the difference.

Both implementations serve their output over HTTP. The same real browser
renders both. We measure the output after each client runtime has taken
control of the observable DOM and rendered it.

## 3. The Corpus

`corpus/` holds the shared set of applications (the corpus). Every project in
it must build under **both** implementations. A project that only one
implementation can build does not belong there. Instead:

- the ledger lists the feature it tests as `rust-only` or `unsupported`;
- that implementation's own test suite proves the feature.

Each project may declare `parity.json`:

```json
{
  "expected_divergences": [
    { "check": "...", "detail": "...", "reason": "...", "direction": "intentional" }
  ]
}
```

Every run reports a declared divergence, but the divergence does not fail the
gate. An undeclared divergence fails the gate. Adding an entry is a deliberate
choice. Every new entry must have a matching ledger row.

## 4. The Ledger

[`PARITY_LEDGER.md`](PARITY_LEDGER.md) gives every feature one of these
statuses: `identical`, `equivalent`, `changed`, `unsupported`, or `rust-only`.

- `identical` requires a corpus project that proves it.
- `equivalent` requires one of these:
  - a corpus project with a declared divergence;
  - a Rust phase suite, when the feature has no legacy counterpart.
- `changed` requires `ty migrate check` to report the construct with an action.
- `unsupported` requires `ty migrate check` to report the construct. The
  report must name what the maintainer must do instead.

Nobody may promote a feature to a new status silently. A promotion requires
the evidence that the new status demands.

## 5. `ty migrate check`

```text
ty migrate check [PROJECT] [--json] [--allow-unsupported]
```

The analysis:

- never modifies the project and never runs project code;
- never enters `.git`, `.tachyon`, `dist`, `dist-bin`, `node_modules`,
  `target`, `__pycache__`, or any folder whose name starts with a dot;
- never follows a symlink;
- reads at most 4 MiB per file and reports at most 10,000 findings;
- classifies each finding as `supported`, `changed`, or `unsupported`;
- adds a required action to every `changed` and `unsupported` finding;
- sorts findings by source and then by feature. As a result, two runs over
  one project produce identical output.

Exit behavior:

| Condition | Exit |
| --- | --- |
| No `unsupported` findings | success |
| Any `unsupported` finding | `TY1702` |
| Any `unsupported` finding with `--allow-unsupported` | success |
| Target cannot be read or is not a directory | `TY1701` |

`--json` prints the report in the Migration Report v1 format instead of the
human-readable summary.

## 6. Failure Behavior

| Code | Condition |
| --- | --- |
| `TY1701` | The migration target cannot be read, is not a directory, or is larger than the analysis limits allow. |
| `TY1702` | The project contains constructs that have no equivalent in this implementation. |

## 7. Exit Gate

- [x] Every corpus project builds under both implementations.
- [x] Every corpus route renders an identical semantic DOM, or its divergence
      is declared and has a ledger row.
- [x] The route graph matches for every corpus project.
- [x] `ty migrate check` classifies the archived migration fixture without running
      it, and every finding that is not supported has an action.
- [x] The ledger covers the view, server, build, and native areas.
- [x] The differential comparison runs in CI.

Phase 6 does not migrate user projects. It tells a maintainer exactly what
the migration of their project would require.
