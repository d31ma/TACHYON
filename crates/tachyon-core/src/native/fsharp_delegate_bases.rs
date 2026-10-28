use super::fsharp_delegate_base::FsharpDelegateBase;
use std::collections::BTreeMap;
use std::ops::Range;

#[derive(Default)]
pub(super) struct FsharpDelegateBases {
    path: String,
    delegate: Option<String>,
    marked: bool,
    marker: Option<Range<usize>>,
    found: Vec<FsharpDelegateBase>,
}

impl FsharpDelegateBases {
    const DIRECTIVE: &'static str = "# 1 \"";
    const QUOTE: char = '"';
    const ATTRIBUTE_OPEN: &'static str = "[<";
    const ATTRIBUTE_CLOSE: &'static str = ">]";
    const ATTRIBUTE_SEPARATOR: char = ';';
    const DELEGATE: &'static str = "Delegate";
    const TYPE: &'static str = "type ";
    const NEEDS: &'static str = "[<Needs>]";
    const MEMBER: &'static str = "member val ";
    const WITH: &'static str = " with ";
    pub(super) const RUNTIME: &'static str = "public static class TacDelegateBase {\n    public static System.Func<string, object> Needed;\n    public static T Need<T>(string service) => (T)Needed(service);\n}\n";
    pub(super) const CONNECTION: &'static str = "static class FsharpDelegateBaseSetup {\n    [System.Runtime.CompilerServices.ModuleInitializer]\n    internal static void Connect() { TacDelegateBase.Needed = Tac.Need; }\n}\n";

    pub(super) fn read(text: &str, entry: &str) -> Vec<FsharpDelegateBase> {
        let start = Self {
            path: entry.to_owned(),
            ..Self::default()
        };
        text.split_inclusive('\n')
            .fold((start, 0), |(mut bases, offset), line| {
                bases.line(line.trim_end_matches(['\r', '\n']), offset);
                (bases, offset + line.len())
            })
            .0
            .found
    }

    pub(super) fn filled(text: &str, instances: &BTreeMap<String, String>) -> String {
        Self::read(text, "")
            .iter()
            .rev()
            .filter_map(|base| Some((base, instances.get(&base.kind)?)))
            .fold(text.to_owned(), |mut written, (base, instance)| {
                written.replace_range(base.value.clone(), &base.need(instance));
                written.replace_range(base.marker.clone(), &" ".repeat(base.marker.len()));
                written
            })
    }

    fn line(&mut self, line: &str, at: usize) {
        let top = !line.starts_with(char::is_whitespace);
        match [line.trim().is_empty(), top] {
            [true, _] => {}
            [false, true] => self.top_line(line),
            [false, false] => self.member_line(line, at),
        }
    }

    fn top_line(&mut self, line: &str) {
        self.follow_directive(line);
        self.delegate = line
            .strip_prefix(Self::TYPE)
            .filter(|_| self.marked)
            .map(Self::type_name);
        let attribute = line.starts_with(Self::ATTRIBUTE_OPEN);
        let delegate = [self.marked, Self::marks_delegate(line)].contains(&true);
        self.marked = [attribute, delegate] == [true, true];
        self.marker = None;
    }

    fn follow_directive(&mut self, line: &str) {
        let path = line
            .strip_prefix(Self::DIRECTIVE)
            .and_then(|rest| rest.split(Self::QUOTE).next());
        self.path = path.map_or_else(|| std::mem::take(&mut self.path), str::to_owned);
    }

    fn type_name(rest: &str) -> String {
        rest.split(|letter: char| [letter.is_alphanumeric(), letter == '_'] == [false, false])
            .next()
            .unwrap_or_default()
            .to_owned()
    }

    fn marks_delegate(line: &str) -> bool {
        line.trim()
            .trim_start_matches(Self::ATTRIBUTE_OPEN)
            .trim_end_matches(Self::ATTRIBUTE_CLOSE)
            .split(Self::ATTRIBUTE_SEPARATOR)
            .filter_map(|attribute| attribute.split('(').next())
            .any(|name| name.trim().rsplit('.').next() == Some(Self::DELEGATE))
    }

    fn member_line(&mut self, line: &str, at: usize) {
        let waiting = self.marker.take();
        let trimmed = line.trim_start();
        let start = at + line.len() - trimmed.len();
        let marks = [self.delegate.is_some(), trimmed.trim_end() == Self::NEEDS];
        self.marker = (marks == [true, true]).then(|| start..start + Self::NEEDS.len());
        let base = waiting.and_then(|marker| self.base(trimmed, start, marker));
        self.found.extend(base);
    }

    fn base(
        &self,
        trimmed: &str,
        start: usize,
        marker: Range<usize>,
    ) -> Option<FsharpDelegateBase> {
        let declared = trimmed.strip_prefix(Self::MEMBER)?;
        let (name, rest) = declared.split_once(':')?;
        let (kind, value) = rest.split_once('=')?;
        let kind = kind.trim();
        (!kind.ends_with(Self::DELEGATE)).then_some(())?;
        let expression = value.trim_start();
        let length = expression.find(Self::WITH)?;
        let begin = start + trimmed.len() - expression.len();
        Some(FsharpDelegateBase {
            delegate: self.delegate.clone()?,
            kind: kind.to_owned(),
            name: name.trim().to_owned(),
            path: self.path.clone(),
            marker,
            value: begin..begin + length,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpDelegateBases;
    use std::collections::BTreeMap;

    struct Assembled;

    impl Assembled {
        const TEXT: &'static str = "[<Service>]\ntype QuoteService() =\n    member val private TaxDelegate : TaxDelegate = new TaxDelegate() with get, set\n    [<Needs>]\n    member val Other : OtherService = Unchecked.defaultof<OtherService> with get, set\n\n# 1 \"client/delegates/TaxDelegate.fs\"\n[<Delegate>]\ntype TaxDelegate() =\n    [<Needs>]\n    member val TaxService : TaxService = Unchecked.defaultof<TaxService> with get, set\n    member val private Label : LabelDelegate = new LabelDelegate() with get, set\n\n    member this.Rate(region: string) : int =\n        this.TaxService.Rate(region) * 2\n";
    }

    #[test]
    fn only_the_needs_property_of_a_delegate_is_its_base() {
        let bases = FsharpDelegateBases::read(Assembled::TEXT, "client/services/QuoteService.fs");
        assert_eq!(bases.len(), 1);
        assert_eq!(bases[0].delegate, "TaxDelegate");
        assert_eq!(bases[0].kind, "TaxService");
        assert_eq!(bases[0].name, "TaxService");
        assert_eq!(bases[0].path, "client/delegates/TaxDelegate.fs");
    }

    #[test]
    fn the_base_property_asks_the_page_for_its_service() {
        let instances = BTreeMap::from([(String::from("TaxService"), String::from("tax"))]);
        let filled = FsharpDelegateBases::filled(Assembled::TEXT, &instances);
        assert!(
            filled.contains("\n             \n    member val TaxService : TaxService = TacDelegateBase.Need<TaxService>(\"tax\") with get, set\n"),
            "{filled}"
        );
        assert!(
            filled.contains("    [<Needs>]\n    member val Other : OtherService"),
            "{filled}"
        );
        assert_eq!(
            filled.lines().count(),
            Assembled::TEXT.lines().count(),
            "{filled}"
        );
        let unknown = FsharpDelegateBases::filled(Assembled::TEXT, &BTreeMap::new());
        assert_eq!(unknown, Assembled::TEXT);
    }
}
