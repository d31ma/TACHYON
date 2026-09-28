# Phase 4 Implementation Plan

## Architecture

Phase 4 adds a `native` module to `tachyon-core`. It does not add a crate.
`NativeCompiler` uses the shared discovery and Tac frontend contracts. It then
plans native routes from the Tac declarations that the author wrote. Native
planning does not use Yon handlers or response bodies. Because of this design,
source validation, component lowering, diagnostics and rollback behavior all
stay on one tested path.

The new data path is:

```text
ProjectDiscovery
  -> Tac frontend + View IR
  -> NativePlanner
  -> Native UI v1 + WebSurface artifacts
  -> MacOsHostGenerator
  -> swiftc
  -> staged .app + manifests
  -> atomic publication
```

The compiler generates WebSurface fallback documents only for the smallest
Tac subtrees that native code does not support. These documents are not Yon
templates or route documents that the server renders.

## Interfaces

```rust
pub enum BuildTarget {
    Web,
    Macos,
}

pub struct NativeBuildOptions {
    pub output_directory: PathBuf,
}

pub struct NativeBuildResult {
    pub output_directory: PathBuf,
    pub application_bundle: PathBuf,
    pub route_count: usize,
    pub native_node_count: usize,
    pub web_surface_count: usize,
    pub sha256: String,
}

pub struct NativeCompiler;

impl NativeCompiler {
    pub fn build(
        project_root: impl AsRef<Path>,
        options: &NativeBuildOptions,
    ) -> Result<NativeBuildResult, Failure>;

    pub async fn build_async(
        project_root: impl AsRef<Path>,
        options: &NativeBuildOptions,
    ) -> Result<NativeBuildResult, Failure>;
}
```

Internal boundaries:

```rust
struct NativePlanner;

impl NativePlanner {
    fn plan(
        route: &str,
        source_path: &str,
        rendered_html: &str,
    ) -> Result<PlannedNativeRoute, Failure>;
}

struct MacOsHostGenerator;

impl MacOsHostGenerator {
    async fn generate(
        application: &NativeApplication,
        routes: &[PlannedNativeRoute],
        web_bundle: &Path,
        stage: &Path,
    ) -> Result<GeneratedMacOsHost, Failure>;
}
```

`tachyon-contracts` contains the canonical serializable types for Native UI
v1, Capability Manifest v1 and Artifact Manifest v1.

## Vertical Slices

1. Add tests that run the compiled binary and fail at first. They cover the
   CLI, output contracts, fallback, accessibility and rollback.
2. Add canonical Rust contract types, and wire tests that check them against
   the schemas.
3. Implement strict native configuration and planning from the resolved tree.
4. Implement validation of declarative state and generation of WebSurface
   documents.
5. Generate deterministic SwiftUI source, resources, Info.plist and manifests.
6. Compile a real macOS application in a staged output, and give it an ad-hoc
   signature.
7. Add the macOS black-box runner. It checks interaction and accessibility, and
   compares screenshots.
8. Update CI (continuous integration), the threat model, the support language,
   the architecture, the release notes and the Phase 4 evidence ledger.

## Recovery and Security

The compiler starts every external tool directly, with fixed arguments. It
limits the size of tool output before the output goes into a diagnostic.
Native publication reuses the existing primitive that stages files, renames
them and rolls back on failure. The previous native output stays unchanged
until all of these steps succeed:

- Swift compilation;
- resource generation;
- contract validation;
- signing;
- manifest hashing.

WebSurface decisions are data. They are not branches in the generated Swift
code. The host decodes Native UI v1 and applies a fixed allowlist of adapters.
Remote content never gets a message handler. Local navigation cannot leave the
application's resource root.

## Explicit Deferrals

Phase 4 does not include these items:

- iOS, Android, Windows, and Linux native hosts;
- third-party native adapter registration;
- arbitrary Tac controller JavaScript on native;
- native capabilities other than deny;
- signed release identities, notarization, installers, updates, and stable
  distribution;
- pixel-identical rendering across different platform control toolkits.
