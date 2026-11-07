use tachyon_contracts::NativeTarget;
use tachyon_core::{NativeBuildResult, native_target_directory};

pub(crate) struct NativeBuildReport;

impl NativeBuildReport {
    pub(crate) fn print(native: NativeTarget, result: &NativeBuildResult) {
        println!(
            "Built {} app hosting {} routes to {} ({}) compiled={} reused={}",
            native_target_directory(native),
            result.route_count(),
            result.application_bundle().display(),
            result.sha256(),
            result.compiled_routes(),
            result.reused_routes()
        );
        result.notes().iter().for_each(|note| println!("{note}"));
    }
}
