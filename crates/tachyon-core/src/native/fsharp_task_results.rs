use crate::Failure;
use crate::companion::FsharpTaskMembers;
use crate::fsharp_source::FsharpView;
use crate::route_types::ModelTable;

#[derive(Default)]
pub(super) struct FsharpTaskResults {
    pub(super) names: Vec<String>,
    pub(super) entries: String,
}

impl FsharpTaskResults {
    const SCALARS: [&'static str; 16] = [
        "string",
        "int",
        "long",
        "short",
        "uint",
        "ulong",
        "ushort",
        "byte",
        "sbyte",
        "double",
        "float",
        "decimal",
        "bool",
        "char",
        "object",
        "BehaviorJSON",
    ];
    const NAMESPACE: &'static str = "System.Threading.Tasks.";
    const CLASS: &'static str = "public class ";
    const MODELS: &'static str = "Models.";
    const LISTS: [&'static str; 2] = ["FSharpList<", "List<"];
    const REFUSAL: &'static str = "An F# browser method returns Task or ValueTask of nothing, BehaviorJSON, a plain value, a model from client/models, or a list or array of these; options, maps and other types are not supported.";

    pub(super) fn read(
        view: &FsharpView,
        (browser, listeners): (bool, &[String]),
        (class, models): (&str, &ModelTable),
    ) -> Result<Self, Failure> {
        let selected = browser.then(|| Self::declared(view, (class, listeners), models));
        selected
            .transpose()
            .map(Option::unwrap_or_default)
            .map_err(|failure| view.failure(&failure))
    }

    fn declared(
        view: &FsharpView,
        (class, listeners): (&str, &[String]),
        models: &ModelTable,
    ) -> Result<Self, Failure> {
        let source = crate::annotation_layout::masked_source(view.path(), view.text())?;
        let mut results = Self::default();
        Self::class_lines(&source, class)
            .into_iter()
            .filter(|line| Self::task_kind(line).is_some())
            .filter(|line| !Self::listens(line, listeners))
            .try_for_each(|line| {
                Self::check(view, &source, line, models)?;
                results.remember(line);
                Ok(())
            })?;
        Ok(results)
    }

    fn class_lines<'a>(source: &'a str, class: &str) -> Vec<&'a str> {
        let header = format!("{}{class} ", Self::CLASS);
        source
            .lines()
            .scan(false, |inside, line| {
                let top = [line.starts_with(Self::CLASS), line.starts_with('[')].contains(&true);
                *inside = [*inside, line.starts_with(&header)][usize::from(top)];
                Some((*inside, line))
            })
            .filter(|(inside, _)| *inside)
            .map(|(_, line)| line)
            .collect()
    }

    fn check(
        view: &FsharpView,
        source: &str,
        line: &str,
        models: &ModelTable,
    ) -> Result<(), Failure> {
        let payload = Self::task_kind(line).and_then(Self::payload);
        let supported = payload.is_none_or(|payload| Self::supported(payload, models));
        supported
            .then_some(())
            .ok_or_else(|| Self::failure(view, source, line))
    }

    fn listens(line: &str, listeners: &[String]) -> bool {
        FsharpTaskMembers::name(line).is_some_and(|name| listeners.contains(&name))
    }

    fn remember(&mut self, line: &str) {
        let Some((name, entry)) = FsharpTaskMembers::entry(line) else {
            return;
        };
        self.names.push(name);
        self.entries.push_str(&entry);
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
        let name = name.strip_prefix(Self::NAMESPACE).unwrap_or(name);
        ["Task", "ValueTask"].contains(&name)
    }

    fn payload(kind: &str) -> Option<&str> {
        let (_, rest) = kind.split_once('<')?;
        Some(rest.strip_suffix('>').unwrap_or(rest).trim())
    }

    fn supported(payload: &str, models: &ModelTable) -> bool {
        let payload = payload.strip_prefix(Self::MODELS).unwrap_or(payload);
        let item = payload
            .strip_suffix("[]")
            .or_else(|| Self::list_item(payload));
        item.map_or_else(
            || Self::single(payload, models),
            |item| Self::supported(item, models),
        )
    }

    fn single(payload: &str, models: &ModelTable) -> bool {
        [Self::SCALARS.contains(&payload), models.is_record(payload)].contains(&true)
    }

    fn list_item(payload: &str) -> Option<&str> {
        Self::LISTS.iter().find_map(|list| {
            payload
                .strip_prefix(list)
                .and_then(|rest| rest.strip_suffix('>'))
                .map(str::trim)
        })
    }

    fn failure(view: &FsharpView, source: &str, line: &str) -> Failure {
        let at = source.find(line).unwrap_or(0);
        Failure::one(crate::failure::diagnostic(
            1405,
            Self::REFUSAL,
            None,
            crate::failure::source_span(&view.path().to_string_lossy(), at, at + line.len()),
        ))
    }
}
