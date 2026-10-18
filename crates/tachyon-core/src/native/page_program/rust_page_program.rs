use super::{PageProgram, PageServices};
use crate::Failure;
use crate::project::NativeCompanion;

pub(super) struct RustPageProgram;

impl RustPageProgram {
    pub(super) fn lowered(services: PageServices<'_, '_>) -> Result<PageProgram, Failure> {
        PageProgram::each_service(NativeCompanion::Rust, services)
    }
}
