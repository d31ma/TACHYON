#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    tachyon_core::fuzzing::managed_resource_headers(data);
});
