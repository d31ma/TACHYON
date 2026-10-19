use super::shared_page_program::SharedPageProgram;
use super::{PageProgram, PageServices};
use crate::Failure;
use crate::project::NativeCompanion;

pub(super) struct SwiftPageProgram;

impl SwiftPageProgram {
    pub(super) fn lowered(services: PageServices<'_, '_>) -> Result<PageProgram, Failure> {
        SharedPageProgram::lowered(NativeCompanion::Swift, services)
    }
}
