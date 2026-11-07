# Phase 3 Implementation Plan

> This is a historical plan. ADR (architecture decision record) 0015 replaces
> its decisions about Tac SSR (server-side rendering) and hydration. ADR 0016
> removes Yon templates, route context and build-time execution.

## Boundary

Phase 3 extends the existing modular monolith. It adds no crate and no internal
service. Each crate has one job:

- `tachyon-contracts` owns the wire types for View IR (intermediate
  representation) and the View Source Map.
- `tachyon-core` owns parsing, validation, component lowering, client-plan
  generation, incremental reuse and the generated browser runtime.
- `tachyon-cli` still only orchestrates the work and presents results.

## Public Interfaces

```rust
pub struct ViewIr {
    pub contract_version: u8,
    pub source: String,
    pub root: ViewNode,
}

pub enum ViewNode {
    Element(ViewElement),
    Text(ViewText),
    Conditional(ViewConditional),
    Iteration(ViewIteration),
    Component(ViewComponent),
}

pub struct ViewSourceMap {
    pub contract_version: u8,
    pub output: String,
    pub sources: Vec<String>,
    pub mappings: Vec<ViewSourceMapping>,
}

pub struct TemplateProgram { ... }

impl TemplateFrontend {
    pub fn compile(
        source: &str,
        source_path: &str,
        components: &ComponentRegistry,
    ) -> Result<TemplateProgram, Failure>;
}

impl ClientViewRenderer {
    pub fn render(
        program: &TemplateProgram,
        components: &ComponentRegistry,
        route: &str,
    ) -> Result<ClientRenderPlan, Failure>;
}

impl WebCompiler {
    pub async fn build_async(
        project_root: impl AsRef<Path>,
        options: &BuildOptions,
    ) -> Result<BuildResult, Failure>;
}
```

`TemplateProgram` holds a validated, immutable AST (abstract syntax tree).

The synchronous `WebCompiler::build` stays for library callers that do not use
async code. It runs the same deterministic Phase 3 pipeline. The compiled CLI
and the development server use `build_async`.

`BuildOptions` gets a new field, `incremental: bool`. Its default is true.
`BuildResult` gets two new methods: `compiled_routes()` and `reused_routes()`.

## Vertical Slices

1. Add typed contracts for View IR and the View Source Map. Then serialize
   validated interpolation ASTs into the client plan.
2. Add the bounded expression parser. Lower the conditional and iteration
   syntax, but do not evaluate it during the build.
3. Discover, parse and lower components. Support properties and slots, check
   for cycles and enforce expansion budgets.
4. Keep Yon handler execution outside the build and the view pipeline.
5. Emit deterministic component mount metadata, component modules and the
   external client renderer.
6. Emit View IR and source maps. Then add verified incremental reuse and atomic
   build state.
7. Add recovery that reports many diagnostics at once, adversarial tests,
   real-browser checks, CI (continuous integration), security documentation and
   completion evidence.

If a slice fails, the previously published output stays unchanged.

## Black-Box Acceptance Corpus

The compiled `ty` executable must prove all of these:

- The build discovers JavaScript and Python Yon handlers without executing
  `GET`. It rejects `yon.html`.
- These render correctly: both control syntaxes, nested loops, escaped
  bindings, dynamic attributes, web components, component properties and slots.
- View IR and source maps validate against their canonical schemas.
- Every component mount policy has deterministic client-plan and module
  behavior.
- A real Chromium session creates the initial DOM, schedules activation,
  handles an interaction and renders structural changes again.
- These fail safely: malformed expressions and controls, component cycles,
  invalid slots and islands, expansion limits and missing values.
- Two independent source errors appear in one Diagnostics v1 report.
- Incremental builds behave correctly:
  - unchanged Tac routes reuse verified output;
  - a component change invalidates the routes that use the component;
  - routes with only a handler emit no view;
  - corrupt state is treated as a safe cache miss;
  - `--no-incremental` forces recompilation.
- A failed build keeps the complete previous output.

## Security and Operational Review

- No template path can lead to JavaScript evaluation or a shell.
- The browser escapes each render value for the context where it is output.
- View IR, source maps, manifests, diagnostics and incremental state do not
  include Yon response values or secrets.
- The compiler generates component module URLs and checks that they are
  same-origin.
- Props are public. A prop cannot authorize a request.
- Handlers never run during a build. At request time they keep the Phase 2
  process and environment limits.
- Cache state is untrusted, contained to its path, verified by digest and safe
  to delete.
- All of these have limits: the parser, the evaluator, expansion, output,
  diagnostics, handlers and cache reads.
