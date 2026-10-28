use super::PageProgram;
use super::shared_page::SharedPage;
use crate::Failure;
use crate::fsharp_source::{FsharpLines, FsharpSource};
use crate::native::fsharp_service::FsharpService;
use crate::native::fsharp_service_needs::FsharpServiceNeeds;
use crate::native::registry::NativeCompanionInput;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Default)]
pub(super) struct FsharpPage<'page> {
    opens: BTreeSet<String>,
    runtimes: BTreeSet<&'static str>,
    page: SharedPage<'page>,
}

impl<'page> FsharpPage<'page> {
    const NAMESPACE: &'static str = FsharpServiceNeeds::PAGE_NAMESPACE;
    const DIRECTIVE: &'static str = "\n# 1 \"";
    const DIRECTIVE_END: &'static str = "\"\n";
    const OPEN: &'static str = "open ";

    pub(super) fn shares(item: &NativeCompanionInput) -> bool {
        [item.web_assembly, FsharpSource::is_fsharp(&item.source)] == [true, true]
    }

    pub(super) fn add(
        &mut self,
        (index, item): (usize, &'page NativeCompanionInput),
        needs: &FsharpServiceNeeds,
    ) -> Result<(), Failure> {
        let captured = crate::native::registry::read_captured_source(&item.source)?;
        let part = FsharpService::part((item, needs), crate::without_bom(&captured))?;
        let mut segments = part.assembled.split(Self::DIRECTIVE);
        let entry = item
            .source
            .strip_prefix(&item.snapshot_root)
            .unwrap_or(&item.source)
            .to_string_lossy()
            .replace('\\', "/");
        self.add_head(segments.next().unwrap_or_default(), &entry)?;
        segments.try_for_each(|segment| self.add_part(segment))?;
        self.runtimes.extend(part.runtimes);
        self.page.add_service(index, item, part.table);
        Ok(())
    }

    pub(super) fn written(self, program: PageProgram) -> Result<PageProgram, Failure> {
        self.page
            .with_first(program, |first, program| self.shared(first, program))
    }

    fn shared(
        &self,
        first: &NativeCompanionInput,
        program: PageProgram,
    ) -> Result<PageProgram, Failure> {
        let assembled = self.assembled();
        let runtime: String = self.runtimes.iter().copied().collect();
        let (setups, fsharp, runtime) = FsharpService::program(first, &assembled, (&runtime, ""))?;
        let mut program = self.page.written(program, Self::NAMESPACE, (&setups, ""));
        program
            .fsharp
            .push_str(&FsharpSource::web_namespace(Self::NAMESPACE, &fsharp));
        program
            .runtimes
            .push_str(&FsharpSource::runtime_namespace(Self::NAMESPACE, &runtime));
        Ok(program)
    }

    fn assembled(&self) -> String {
        let opens: Vec<&str> = self.opens.iter().map(String::as_str).collect();
        let parts: Vec<String> = self
            .page
            .parts()
            .into_iter()
            .map(|(path, text)| FsharpLines::located(Path::new(path), text))
            .collect();
        format!("{}\n{}", opens.join("\n"), parts.join("\n"))
    }

    fn add_head(&mut self, head: &str, entry: &str) -> Result<(), Failure> {
        self.opens.extend(
            head.lines()
                .filter(|line| line.starts_with(Self::OPEN))
                .map(|line| line.trim().to_owned()),
        );
        let text: Vec<&str> = head
            .lines()
            .map(|line| [line, ""][usize::from(line.starts_with(Self::OPEN))])
            .collect();
        let text = text.join("\n");
        (!text.trim().is_empty())
            .then(|| self.page.add_part(entry, &text))
            .transpose()
            .map(drop)
    }

    fn add_part(&mut self, segment: &str) -> Result<(), Failure> {
        let (path, text) = segment
            .split_once(Self::DIRECTIVE_END)
            .unwrap_or((segment, ""));
        self.page.add_part(path, text)
    }
}
