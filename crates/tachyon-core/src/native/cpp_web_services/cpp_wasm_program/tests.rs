use super::CppWasmProgram;
use std::path::Path;

struct Programs;

impl Programs {
    const SERVICE: &'static str = "#pragma once\n#include <string>\n\nclass [[Service]] CounterService {\npublic:\n    int count = 0;\n    std::string label = \"tea\";\n\n    int increment() {\n        count += 1;\n        return count;\n    }\n};\n";

    fn composed(root: &Path) -> String {
        let path = root.join("client/services/CounterService.hpp");
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, Self::SERVICE).expect("service");
        let chosen = [(
            String::from("counter"),
            String::from("CounterService"),
            path,
        )];
        crate::tac_cpp::CppProgram::read(root, &chosen)
            .expect("program")
            .web_source()
    }

    fn emscripten_ready() -> bool {
        let compiler = CppWasmProgram::compiler();
        let found =
            crate::program_probe::ProgramProbe::answers(&compiler.to_string_lossy(), "--version");
        assert!(
            found || std::env::var_os("CI").is_none(),
            "em++ is installed in CI, so this test must run there instead of skipping"
        );
        found
    }

    async fn built(program: &str) -> Vec<u8> {
        let cache = tempfile::tempdir().expect("cache");
        CppWasmProgram::built(
            &CppWasmProgram::compiler(),
            CppWasmProgram::RELEASE_LEVEL,
            program,
            &cache.path().join("entry"),
        )
        .await
        .expect("the C++ services build")
        .binary
    }
}

#[tokio::test]
async fn a_repeated_cpp_build_gives_the_same_wasm_bytes() {
    if !Programs::emscripten_ready() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let program = Programs::composed(root.path());
    let first = Programs::built(&program).await;
    let second = Programs::built(&program).await;
    assert_eq!(first.get(..4), Some(&b"\0asm"[..]));
    assert!(
        first == second,
        "two builds of one program differ: {} and {} bytes",
        first.len(),
        second.len()
    );
}

#[test]
fn the_cpp_cache_key_follows_the_compiler_the_level_and_the_program() {
    let key = CppWasmProgram::cache_key(b"emcc 6.0.10", "-Oz", "int main() {}");
    assert_eq!(
        key,
        CppWasmProgram::cache_key(b"emcc 6.0.10", "-Oz", "int main() {}")
    );
    assert_ne!(
        key,
        CppWasmProgram::cache_key(b"emcc 6.0.12", "-Oz", "int main() {}")
    );
    assert_ne!(
        key,
        CppWasmProgram::cache_key(b"emcc 6.0.10", "-O1", "int main() {}")
    );
    assert_ne!(
        key,
        CppWasmProgram::cache_key(b"emcc 6.0.10", "-Oz", "int main() { return 1; }")
    );
}
