use crate::Failure;
use crate::fsharp_source::FsharpView;

pub(super) struct FsharpTaskResults;

impl FsharpTaskResults {
    pub(super) fn require(view: &FsharpView, browser: bool) -> Result<(), Failure> {
        Self::selected(view, browser)
    }

    fn selected(view: &FsharpView, browser: bool) -> Result<(), Failure> {
        if browser {
            Self::declared(view).map_err(|failure| view.failure(&failure))
        } else {
            Ok(())
        }
    }

    fn declared(view: &FsharpView) -> Result<(), Failure> {
        let source = crate::annotation_layout::masked_source(view.path(), view.text())?;
        for line in source.lines() {
            Self::signature(view, &source, line)?;
        }
        Ok(())
    }

    fn signature(view: &FsharpView, source: &str, line: &str) -> Result<(), Failure> {
        for kind in Self::task_kind(line).into_iter() {
            Self::result(view, source, line, kind)?;
        }
        Ok(())
    }

    fn task_kind(line: &str) -> Option<&str> {
        let mut words = line.split_whitespace();
        words.next().filter(|word| *word == "public")?;
        words
            .find(|word| !["async", "static", "virtual", "override"].contains(word))
            .filter(|kind| Self::task_name(kind))
    }

    fn task_name(kind: &str) -> bool {
        let name = kind.split('<').next().unwrap_or(kind);
        [
            "Task",
            "System.Threading.Tasks.Task",
            "ValueTask",
            "System.Threading.Tasks.ValueTask",
        ]
        .contains(&name)
    }

    fn result(view: &FsharpView, source: &str, line: &str, kind: &str) -> Result<(), Failure> {
        let supported = [
            "Task",
            "System.Threading.Tasks.Task",
            "Task<BehaviorJSON>",
            "System.Threading.Tasks.Task<BehaviorJSON>",
        ]
        .contains(&kind);
        if !supported {
            let at = source.find(line).unwrap_or(0);
            return Err(Failure::one(crate::failure::diagnostic(
                1405,
                "F# browser continuations currently require Task or Task<BehaviorJSON>; model, container and ValueTask results remain unsupported.",
                None,
                crate::failure::source_span(&view.path().to_string_lossy(), at, at + line.len()),
            )));
        }
        Ok(())
    }
}
