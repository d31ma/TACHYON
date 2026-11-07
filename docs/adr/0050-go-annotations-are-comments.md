# ADR 0050: Go Annotations Are Comments

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0045](0045-go-is-a-service-language.md) (Go annotation spelling)
- Preserves: [ADR 0046](0046-tachyon-annotations-are-global.md) (no annotation
  imports), [ADR 0047](0047-annotation-names-are-plain-english.md) (names), and
  [ADR 0048](0048-yon-routes-come-from-annotations.md) (controller routes)

## Context

Go has no decorator syntax. Tachyon originally used Rust-style `#[Name]`
markers. Both that spelling and a bare `@Name` stop Go tools from parsing
the source. Comment markers carry the same information without changing Go
syntax. The preparation probes found that standalone `// @Name` comments
keep their spelling under `gofmt`, including type, method, field, parameter,
stacked and strict-JSON cases. A self-contained annotated fixture also passes
`go vet`. The same probes pass with Go 1.27.1 on Linux ARM64 in WSL.

## Decision

- A Go marker is `// @Name` or `// @Name({...})`, on its own line immediately
  before the declaration. There is exactly one space after `//`.
- The existing `yon::` and `tac::` namespaces remain available, with the same
  annotation names and arguments as before. Arguments are not Go expressions.
- A marked parameter occupies its own line in a multiline parameter list.
  Normal Go comma rules apply.
- The scanner recognizes markers only in line comments outside strings, raw
  strings and block comments. Ordinary comments are unchanged.
- A `// @Name` comment is a marker when Name is a Tachyon annotation (exact
  spelling), an old or foreign name that Tachyon explains (such as
  `@Autowired` or `@ControllerAdvice`), or a namespaced name. A name that is
  close to a Tachyon annotation (a typo such as `@GetRote` or `@Getroute`) is
  refused with "did you mean `// @GetRoute`?". Every other `// @Word` comment
  stays an ordinary comment, so documentation tools such as swaggo/swag
  (`// @Summary`, `// @Param`, `// @Router`, `// @Description text`,
  `// @ID name`) work in the same files. Lowercase documentation tags remain
  comments. (Amended 2026-09-26 by user decision; the first version refused
  every unknown capitalized name.)
- A marker line holds only `@Name` and, optionally, one argument group that
  opens and closes on that line. Text after the marker, arguments that span
  lines, and `//@Name` or `//  @Name` spacing are refused on that line.
- Marker removal preserves the author's line numbers.
- Go's old `#[Name]` spelling is a build error naming the comment spelling.
  `ty migrate fix --write` converts Go markers only. It retains the migration
  command's no-overwrite and symlink guards and is idempotent.
- This applies to Yon controllers, providers, workers, middleware and test
  units, and to Tac Go services and test units. Models remain data-only and
  cannot gain behavior by using a comment marker.

## Consequences

Annotated source is valid Go syntax. `gofmt`, `go vet` and editor parsers no
longer fail because of Tachyon marker syntax. This does not supply missing
framework types to an editor: semantic checking still needs every referenced
declaration. Tachyon continues to generate those declarations for builds.

PHP and Rust keep their attribute spelling. Go examples use comment markers
and ordinary Go formatting. Tests cover lexical boundaries, refusals,
migration, formatting, runtime lowering and browser WebAssembly execution.
