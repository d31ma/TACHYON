#![no_main]

use libfuzzer_sys::fuzz_target;
use tachyon_contracts::NativeTarget;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let targets = [
        NativeTarget::Linux,
        NativeTarget::Macos,
        NativeTarget::Windows,
        NativeTarget::Android,
        NativeTarget::Ios,
    ];
    let first_is_valid = tachyon_core::fuzzing::plan_native(targets[0], source).is_ok();
    for target in &targets[1..] {
        let is_valid = tachyon_core::fuzzing::plan_native(*target, source).is_ok();
        assert_eq!(is_valid, first_is_valid);
    }
});
