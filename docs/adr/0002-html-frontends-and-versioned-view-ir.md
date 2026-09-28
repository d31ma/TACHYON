# ADR 0002: HTML Frontends and Versioned View IR

- Status: Accepted
- Date: 2026-07-26
- Amended by: ADR 0016 removes `yon.html` as a frontend; this decision now
  applies only to Tac views.

## Context

Application developers must use HTML for views on web and non-web targets.
Tachyon also needs:

- structural control flow;
- components;
- islands;
- diagnostics that point to source locations;
- deterministic builds;
- native code generation.

If every backend received raw HTML, each backend would parse it again. The
behavior of control tags, components and fallbacks would then depend on the
platform.

## Decision

`tac.html` is the entry point of the HTML frontend. The frontend:

1. parses the file without running application code;
2. keeps source spans (the location of each piece of source);
3. resolves control tags and components;
4. lowers the validated source into View IR (intermediate representation).

`if`, `else`, `for` and `loop` are structural compiler syntax. They do not
remain as element nodes.

View IR v1 is a canonical, platform-neutral contract for machines. Its JSON
Schema defines interchange and compatibility. Optimized internal Rust
representations may be different. Conformance tests must then prove that they
behave the same.

The same canonical source, configuration, compiler version and target inputs
must produce byte-identical View IR.

## Consequences

- Web and native backends share parsing and semantic validation.
- Diagnostics can name stable source spans and codes.
- New backends read one normalized representation.
- Every IR change must state its compatibility and migration rules.
- The parser, the component resolver and the expression language stay separate
  decisions for later.

## Rejected Alternatives

- **Raw DOM as the compiler contract**: it loses Tachyon's structural meaning.
  It is also not stable across parser implementations.
- **Platform-specific parsing**: each platform could interpret the source
  differently. It also gives attackers more code to target.
- **A new JSX-like view language**: it breaks the HTML-only contract for
  developers.
- **Keeping control tags as unknown elements**: browsers and native renderers
  would then behave incorrectly.

## Acceptance Gate

- View IR v1 has a valid canonical schema and positive/negative fixtures;
- the HTML-only and control-tag invariants appear in `CONTEXT.md`;
- the docs describe web, server and native backends as consumers of View IR,
  not as other parsers;
- any future parser proves deterministic source-to-IR output with golden
  fixtures.
