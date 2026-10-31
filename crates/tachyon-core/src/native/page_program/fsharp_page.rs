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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use crate::native::registry::{self, NativeCompanionInput};
    use crate::project::NativeCompanion;
    use std::path::Path;

    struct FsharpPageCheck;

    impl FsharpPageCheck {
        const FILES: [(&'static str, &'static str); 3] = [
            (
                "client/setups/PageProgramSetup.fs",
                "[<Setup>]\ntype PageProgramSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        execution.Check()\n        let adapters: BehaviorAdapters = BehaviorAdapters()\n        adapters.http.Add(\"pageprogram\", new ProgramHttpDelegate())\n        adapters\n",
            ),
            (
                "client/delegates/ProgramHttpDelegate.fs",
                "open System.Collections.Generic\nopen System.Threading.Tasks\n\n[<Delegate>]\ntype ProgramHttpDelegate() =\n    let identity = System.Guid.NewGuid().ToString()\n    let mutable count = 0\n\n    interface BehaviorHTTPAdapter with\n        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =\n            execution.Check()\n            count <- count + (if request.Get(\"method\").Text = \"POST\" then 1 else 0)\n            let state = Dictionary<string, BehaviorJSON>()\n            state[\"identity\"] <- BehaviorJSON.From(identity)\n            state[\"count\"] <- BehaviorJSON.From(count)\n            Task.FromResult(BehaviorJSON.Object(state))\n",
            ),
            (
                "client/remotes/PageProgramClient.fs",
                "open System.Threading.Tasks\n\n[<HTTPClient({\"adapter\":\"pageprogram\",\"path\":\"/program\",\"setup\":\"PageProgramSetup\"})>]\ntype PageProgramClient =\n    [<GetRequest(\"/read\")>]\n    abstract read: unit -> Task<BehaviorJSON>\n\n    [<PostRequest(\"/increment\")>]\n    abstract increment: unit -> Task<BehaviorJSON>\n",
            ),
        ];
        const SERVICES: [(&'static str, &'static str); 2] =
            [("writer", "WriterService"), ("reader", "ReaderService")];

        fn service(class: &str) -> String {
            format!(
                "open System.Threading.Tasks\n\n[<Service>]\ntype {class}() =\n    [<Needs>]\n    member val PageProgramClient : PageProgramClient = Unchecked.defaultof<PageProgramClient> with get, set\n\n    member this.Read() : Task<BehaviorJSON> =\n        this.PageProgramClient.read ()\n"
            )
        }

        fn write(root: &Path, relative: &str, text: &str) {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, text).expect("file");
        }

        fn source() -> String {
            let root = tempfile::tempdir().expect("project");
            for (relative, text) in Self::FILES {
                Self::write(root.path(), relative, text);
            }
            let mut services = Vec::new();
            for (instance, class) in Self::SERVICES {
                let relative = format!("client/services/{class}.fs");
                Self::write(root.path(), &relative, &Self::service(class));
                services.push((
                    instance.to_owned(),
                    class.to_owned(),
                    root.path().join(relative),
                ));
            }
            let companions =
                NativeCompanionInput::web_services(root.path(), NativeCompanion::CSharp, &services);
            registry::source(&companions, NativeCompanion::CSharp)
                .expect("the page lowers")
                .expect("a .NET program")
        }

        fn assert_counted(source: &str, expected: &[(&str, usize)]) {
            for (text, count) in expected {
                assert_eq!(source.matches(text).count(), *count, "{text}:\n{source}");
            }
        }
    }

    #[test]
    fn two_fsharp_services_share_one_page_namespace() {
        let source = FsharpPageCheck::source();
        FsharpPageCheck::assert_counted(
            &source,
            &[
                ("namespace rec TachyonServiceFsharp\n", 1),
                ("type PageProgramSetup", 1),
                ("type ProgramHttpDelegate", 1),
                ("type PageProgramClient", 1),
                ("type WriterService", 1),
                ("type ReaderService", 1),
                ("namespace rec TachyonService0\n", 0),
                ("namespace rec TachyonService1\n", 0),
            ],
        );
        let namespace = source
            .find("namespace rec TachyonServiceFsharp\n")
            .expect("page namespace");
        let setup = source.find("type PageProgramSetup").expect("setup");
        let writer = source.find("type WriterService").expect("writer");
        let reader = source.find("type ReaderService").expect("reader");
        assert!(namespace < setup, "{source}");
        assert!(namespace < writer, "{source}");
        assert!(namespace < reader, "{source}");
    }
}
