use super::{PageProgram, PageServices};
use crate::Failure;
use crate::project::NativeCompanion;

pub(super) struct DotnetPageProgram;

impl DotnetPageProgram {
    pub(super) fn lowered(services: PageServices<'_, '_>) -> Result<PageProgram, Failure> {
        PageProgram::each_service(NativeCompanion::CSharp, services)
    }
}
