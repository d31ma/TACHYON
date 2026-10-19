use super::csharp_page::CsharpPage;
use super::fsharp_page::FsharpPage;
use super::{PageProgram, PageServices};
use crate::Failure;
use crate::native::fsharp_service_needs::FsharpServiceNeeds;
use crate::native::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::collections::BTreeSet;

pub(super) struct DotnetPageProgram;

type DotnetPages<'a, 'page> = (&'a mut CsharpPage<'page>, &'a mut FsharpPage<'page>);

impl DotnetPageProgram {
    pub(super) fn lowered(
        (selected, needs, imports): PageServices<'_, '_>,
    ) -> Result<PageProgram, Failure> {
        let mut csharp = CsharpPage::default();
        let mut fsharp = FsharpPage::default();
        let program = selected.iter().enumerate().try_fold(
            PageProgram::default(),
            |program, (index, item)| {
                Self::with_service(
                    program,
                    (index, item),
                    (&mut csharp, &mut fsharp),
                    (needs, imports),
                )
            },
        )?;
        fsharp.written(csharp.written(program)?)
    }

    fn with_service<'page>(
        program: PageProgram,
        (index, item): (usize, &'page NativeCompanionInput),
        (csharp, fsharp): DotnetPages<'_, 'page>,
        (needs, imports): (&FsharpServiceNeeds, &mut BTreeSet<String>),
    ) -> Result<PageProgram, Failure> {
        let shared = [CsharpPage::shares(item), FsharpPage::shares(item)];
        match shared {
            [true, _] => csharp.add((index, item))?,
            [_, true] => fsharp.add((index, item), needs)?,
            _ => {
                return program.with_service(
                    NativeCompanion::CSharp,
                    (index, item),
                    needs,
                    imports,
                );
            }
        }
        Ok(program)
    }
}
