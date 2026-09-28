#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&language, source)) = data.split_first() else {
        return;
    };
    let Ok(source) = std::str::from_utf8(source) else {
        return;
    };
    let extensions = ["js", "ts", "py", "php", "java", "kt", "cs", "rs", "swift"];
    let extension = extensions[usize::from(language) % extensions.len()];
    if extension == "swift" {
        let _ = tachyon_core::fuzzing::compile_tac_annotations(source, extension);
        return;
    }
    let _ = tachyon_core::fuzzing::compile_yon_annotations(source, extension);
    if matches!(extension, "js" | "ts" | "kt" | "cs" | "rs") {
        let _ = tachyon_core::fuzzing::compile_tac_annotations(source, extension);
    }
});
