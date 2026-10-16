use super::{
    AttributeLine, FieldNeed, FsharpLexer, FsharpParameters, FsharpType, LineKind, TypeHeader,
};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct FieldNeeds {
    marker: &'static str,
    needs: Vec<FieldNeed>,
    misplaced: Option<usize>,
    owner: Option<usize>,
    markers: Vec<usize>,
    marked: bool,
    named: Option<String>,
}

impl FieldNeeds {
    const NEEDS: &'static str = "Needs";
    const NAMED: &'static str = "Named";
    const MISPLACED: &'static str = "In F#, Needs marks a settable property with a written type: [<Needs>] member val OrderStore : OrderStore = Unchecked.defaultof<OrderStore> with get, set (ADR 0073)";
    const TWICE: &'static str = "{type} takes {role} twice. Take a role either as a primary constructor parameter or as one Needs property (ADR 0073)";

    pub(crate) fn moved(text: &str) -> String {
        let found = Self::read(text);
        let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
        found.blank_markers(&mut lines);
        found.add_parameters(&mut lines);
        lines.join("\n")
    }

    pub(crate) fn refuse(source: &Path, contents: &str) -> Result<(), Failure> {
        let text = FsharpLexer::without_comments(contents);
        let found = Self::read(&text);
        let lines: Vec<&str> = text.split('\n').collect();
        let problem = found
            .misplaced
            .map(|line| (line, Self::MISPLACED.to_owned()))
            .or_else(|| found.twice(&lines));
        problem.map_or(Ok(()), |(line, message)| {
            Err(Self::failure(source, &lines, line, &message))
        })
    }

    pub(crate) fn constructed(contents: &str, csharp: &str) -> String {
        let text = FsharpLexer::without_comments(contents);
        let lines: Vec<&str> = text.split('\n').collect();
        Self::read(&text).construct_owners(&lines, csharp)
    }

    pub(crate) fn marked_properties(
        contents: &str,
        marker: &'static str,
    ) -> Vec<(String, Vec<(String, String)>)> {
        let text = FsharpLexer::without_comments(contents);
        let lines: Vec<&str> = text.split('\n').collect();
        let found = Self::read_marked(&text, marker);
        found
            .owners()
            .into_iter()
            .map(|(owner, needs)| {
                let name = TypeHeader::read(lines[owner].trim(), &[], false).name;
                let properties = needs
                    .iter()
                    .map(|need| (need.name.clone(), need.provider.clone()))
                    .collect();
                (name, properties)
            })
            .collect()
    }

    fn read(text: &str) -> Self {
        Self::read_marked(text, Self::NEEDS)
    }

    fn read_marked(text: &str, marker: &'static str) -> Self {
        let mut found = Self {
            marker,
            needs: Vec::new(),
            misplaced: None,
            owner: None,
            markers: Vec::new(),
            marked: false,
            named: None,
        };
        for (index, line) in text.split('\n').enumerate() {
            found.line(index, line);
        }
        found
    }

    fn line(&mut self, index: usize, line: &str) {
        match LineKind::of(line.trim()) {
            LineKind::Blank => {}
            LineKind::Attribute => self.attribute(index, line),
            LineKind::Type => self.open_type(index),
            _ => self.member(index, line.trim()),
        }
    }

    fn open_type(&mut self, index: usize) {
        self.owner = Some(index);
        self.reset();
    }

    fn attribute(&mut self, index: usize, line: &str) {
        let indented = line.starts_with(char::is_whitespace);
        let attributes = AttributeLine::read(line.trim());
        let needs = attributes
            .iter()
            .any(|attribute| attribute.name == self.marker);
        let named = Self::named(&attributes);
        let marks = [needs, named.is_some()].contains(&true);
        self.markers
            .extend(([indented, marks] == [true, true]).then_some(index));
        self.marked |= [indented, needs] == [true, true];
        self.named = named.or_else(|| self.named.take());
    }

    fn named(attributes: &[AttributeLine]) -> Option<String> {
        attributes
            .iter()
            .find(|attribute| attribute.name == Self::NAMED)
            .and_then(|attribute| attribute.csharp.split('"').nth(1).map(str::to_owned))
    }

    fn member(&mut self, index: usize, trimmed: &str) {
        let markers = std::mem::take(&mut self.markers);
        let named = self.named.take();
        let owner = self.owner;
        let read = self
            .marked
            .then(|| owner.and_then(|owner| FieldNeed::read(trimmed, owner, markers, named)));
        let misplaced = read.as_ref().filter(|need| need.is_none()).map(|_| index);
        self.misplaced = self.misplaced.or(misplaced);
        self.needs.extend(read.flatten());
        self.reset();
    }

    fn reset(&mut self) {
        self.markers.clear();
        self.marked = false;
        self.named = None;
    }

    fn owners(&self) -> Vec<(usize, &[FieldNeed])> {
        self.needs
            .chunk_by(|first, second| first.owner == second.owner)
            .map(|needs| (needs[0].owner, needs))
            .collect()
    }

    fn blank_markers(&self, lines: &mut [String]) {
        for marker in self.needs.iter().flat_map(|need| &need.markers) {
            lines[*marker] = " ".repeat(lines[*marker].len());
        }
    }

    fn add_parameters(&self, lines: &mut [String]) {
        for (owner, needs) in self.owners() {
            let parameters: Vec<String> = needs.iter().map(FieldNeed::parameter).collect();
            lines[owner] = Self::with_parameters(&lines[owner], &parameters);
        }
    }

    fn construct_owners(&self, lines: &[&str], csharp: &str) -> String {
        let mut written = csharp.to_owned();
        for (owner, needs) in self.owners() {
            let name = TypeHeader::read(lines[owner].trim(), &[], false).name;
            let fields: Vec<&str> = needs.iter().map(|need| need.name.as_str()).collect();
            written = Self::construct(&written, &name, &fields);
        }
        written
    }

    fn twice(&self, lines: &[&str]) -> Option<(usize, String)> {
        self.owners()
            .into_iter()
            .find_map(|(owner, needs)| Self::repeated(lines[owner].trim(), needs))
    }

    fn repeated(header: &str, needs: &[FieldNeed]) -> Option<(usize, String)> {
        let mut taken = TypeHeader::constructor_types(header);
        let repeated = needs.iter().find(|need| {
            let seen = taken.contains(&need.provider);
            taken.push(need.provider.clone());
            seen
        })?;
        let message = Self::TWICE
            .replace("{type}", &TypeHeader::read(header, &[], false).name)
            .replace("{role}", &repeated.provider);
        Some((repeated.markers[0], message))
    }

    fn failure(source: &Path, lines: &[&str], line: usize, message: &str) -> Failure {
        let portable = source.to_string_lossy().replace('\\', "/");
        let start: usize = lines[..line].iter().map(|text| text.len() + 1).sum();
        Failure::one(diagnostic(
            2015,
            message,
            None,
            source_span(&portable, start, start + lines[line].len()),
        ))
    }

    fn with_parameters(line: &str, parameters: &[String]) -> String {
        let opening = line.find('(');
        let closing = opening.and_then(|at| FsharpParameters::closing(&line[at..]));
        let (Some(open), Some(length)) = (opening, closing) else {
            return line.to_owned();
        };
        let close = open + length;
        let written = Some(line[open + 1..close].trim()).filter(|text| !text.is_empty());
        let all: Vec<&str> = written
            .into_iter()
            .chain(parameters.iter().map(String::as_str))
            .collect();
        format!(
            "{}({}){}",
            &line[..open],
            all.join(", "),
            &line[close + 1..]
        )
    }

    fn construct(csharp: &str, name: &str, fields: &[&str]) -> String {
        let opening = format!("new {name}(");
        let mut pieces = csharp.split(&opening);
        let first = pieces.next().unwrap_or_default();
        let rest: String = pieces
            .map(|piece| Self::rebuilt(piece, name, fields))
            .collect();
        format!("{first}{rest}")
    }

    fn rebuilt(piece: &str, name: &str, fields: &[&str]) -> String {
        let opened = format!("({piece}");
        FsharpParameters::closing(&opened).map_or_else(
            || format!("new {name}{opened}"),
            |close| {
                let arguments = &opened[1..close];
                format!(
                    "{}{}",
                    Self::initialized(name, arguments, fields),
                    &opened[close + 1..]
                )
            },
        )
    }

    fn initialized(name: &str, arguments: &str, fields: &[&str]) -> String {
        let arguments: Vec<&str> = FsharpType::split_top_level(arguments, ",")
            .into_iter()
            .filter(|argument| !argument.is_empty())
            .collect();
        let kept = arguments.len().saturating_sub(fields.len());
        let set: Vec<String> = fields
            .iter()
            .zip(&arguments[kept..])
            .map(|(field, argument)| format!("{field} = {argument}"))
            .collect();
        format!(
            "new {name}({}) {{ {} }}",
            arguments[..kept].join(", "),
            set.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::FieldNeeds;
    use std::path::Path;

    const SERVICE: &str = "[<Service>]\ntype OrderService(clock: Clock) =\n    [<Needs>]\n    [<Named(\"FastStore\")>]\n    member val OrderStore : OrderStore = Unchecked.defaultof<OrderStore> with get, set\n\n    [<Needs>]\n    member val Audit : AuditService = Unchecked.defaultof<AuditService> with get, set\n";

    fn refused(source: &str) -> Option<String> {
        FieldNeeds::refuse(Path::new("server/services/OrderService.fs"), source)
            .err()
            .map(|failure| failure.to_string())
    }

    #[test]
    fn needs_properties_read_as_constructor_parameters() {
        let moved = FieldNeeds::moved(SERVICE);
        let lines: Vec<&str> = moved.split('\n').collect();
        assert_eq!(
            lines[1],
            "type OrderService(clock: Clock, orderStore: FastStore, audit: AuditService) ="
        );
        assert_eq!(lines[2].trim(), "");
        assert_eq!(lines[3].trim(), "");
        assert!(lines[4].contains("member val OrderStore"));
        assert_eq!(lines.len(), SERVICE.split('\n').count());
        let plain = FieldNeeds::moved(
            "[<Service>]\ntype A() =\n    [<Needs>]\n    member val B : B = Unchecked.defaultof<B> with get, set\n",
        );
        assert!(
            plain.starts_with("[<Service>]\ntype A(b: B) =\n"),
            "{plain}"
        );
    }

    #[test]
    fn the_container_sets_needs_properties_after_the_constructor() {
        let glue = "if (p0 == null) p0 = new OrderService(resolve_1(), resolve_2(), resolve_3());\nreturn new Other(resolve_1());";
        let constructed = FieldNeeds::constructed(SERVICE, glue);
        assert_eq!(
            constructed,
            "if (p0 == null) p0 = new OrderService(resolve_1()) { OrderStore = resolve_2(), Audit = resolve_3() };\nreturn new Other(resolve_1());"
        );
        let tac = FieldNeeds::constructed(
            "[<Service>]\ntype A() =\n    [<Needs>]\n    member val Publisher : ApplicationEventPublisher = null with get, set\n",
            "var Instance = new A(new ApplicationEventPublisher());",
        );
        assert_eq!(
            tac,
            "var Instance = new A() { Publisher = new ApplicationEventPublisher() };"
        );
    }

    #[test]
    fn needs_on_anything_but_a_settable_typed_property_is_refused() {
        assert!(refused(SERVICE).is_none());
        let field = "[<Service>]\ntype AuditService() =\n    [<Needs>]\n    [<DefaultValue>]\n    val mutable store : AuditStore\n";
        let refusal = refused(field).unwrap_or_default();
        assert!(refusal.contains("settable property"), "{refusal}");
        let read_only = "[<Service>]\ntype A() =\n    [<Needs>]\n    member val B : B = Unchecked.defaultof<B> with get\n";
        assert!(refused(read_only).is_some());
    }

    #[test]
    fn a_role_taken_twice_is_refused() {
        let twice = "[<Service>]\ntype OrderService(store: OrderStore) =\n    [<Needs>]\n    member val Store : OrderStore = Unchecked.defaultof<OrderStore> with get, set\n";
        let refusal = refused(twice).unwrap_or_default();
        assert!(
            refusal.contains("OrderService takes OrderStore twice"),
            "{refusal}"
        );
        let named = "[<Service>]\ntype OrderService(store: FastStore) =\n    [<Needs>]\n    [<Named(\"FastStore\")>]\n    member val Store : IStore = Unchecked.defaultof<IStore> with get, set\n";
        assert!(refused(named).is_some());
    }
}
