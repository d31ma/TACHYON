use super::FsharpType;

pub(crate) struct MemberLine;

impl MemberLine {
    const HIDDEN_PREFIX: &'static str = "__tachyon_";
    const NAME_ENDS: [char; 3] = ['(', ' ', ':'];
    const ABSTRACT_WORDS: [&'static str; 2] = ["abstract", "member"];
    const UNNAMED: &'static str = "value";
    const UNIT: &'static str = "unit";
    const NAME_STARTS: [char; 2] = ['.', ' '];

    pub(crate) fn hidden_name(name: &str) -> String {
        format!("{}{name}", Self::HIDDEN_PREFIX)
    }

    pub(crate) fn renamed(line: &str, name: &str) -> Option<String> {
        let at = Self::name_start(line, name)?;
        let end = at + name.len();
        Some(format!(
            "{}{}{}",
            &line[..at],
            Self::hidden_name(name),
            &line[end..]
        ))
    }

    pub(crate) fn signature(line: &str, name: &str) -> Option<String> {
        let start = Self::name_start(line, name)? + name.len();
        let after = &line[start..];
        let equals = FsharpType::split_top_level(after, "=");
        let written = equals.first().copied().filter(|_| equals.len() > 1)?;
        Some(written.trim_end().to_owned())
    }

    pub(crate) fn is_abstract(line: &str) -> bool {
        line.split_whitespace().next() == Some(Self::ABSTRACT_WORDS[0])
    }

    pub(crate) fn is_static(line: &str) -> bool {
        line.split_whitespace().next() == Some("static")
    }

    pub(crate) fn result(line: &str) -> Option<String> {
        let declaration = Self::without_abstract_words(line.trim_start());
        let (_, signature) = declaration.split_once(':')?;
        let parts = FsharpType::split_top_level(signature, "->");
        parts.last().map(|result| result.trim().to_owned())
    }

    pub(crate) fn concrete(line: &str, body: &str) -> Option<String> {
        let indent = &line[..line.len() - line.trim_start().len()];
        let declaration = Self::without_abstract_words(line.trim_start());
        let (name, signature) = declaration.split_once(':')?;
        let parts = FsharpType::split_top_level(signature, "->");
        let (result, parameters) = parts.split_last()?;
        Some(format!(
            "{indent}member this.{}({}) : {result} = {body}",
            name.trim(),
            Self::parameters(parameters)
        ))
    }

    fn without_abstract_words(declaration: &str) -> &str {
        let mut rest = declaration;
        for word in Self::ABSTRACT_WORDS {
            rest = rest
                .strip_prefix(word)
                .filter(|after| after.starts_with(' '))
                .map_or(rest, str::trim_start);
        }
        rest
    }

    pub(crate) fn pieces<'p>(parts: &[&'p str]) -> Vec<&'p str> {
        parts
            .iter()
            .flat_map(|part| FsharpType::split_top_level(part, "*"))
            .filter(|piece| *piece != Self::UNIT)
            .collect()
    }

    fn parameters(parts: &[&str]) -> String {
        let written: Vec<String> = Self::pieces(parts)
            .into_iter()
            .enumerate()
            .map(|(index, piece)| Self::parameter(piece, index))
            .collect();
        written.join(", ")
    }

    fn parameter(piece: &str, index: usize) -> String {
        let piece = piece.trim();
        let named = FsharpType::split_top_level(piece, ":").len() > 1;
        if named {
            return piece.to_owned();
        }
        format!("{}{index}: {piece}", Self::UNNAMED)
    }

    fn name_start(line: &str, name: &str) -> Option<usize> {
        line.match_indices(name)
            .map(|(at, _)| at)
            .find(|at| Self::is_member_name(line, *at, name))
    }

    fn is_member_name(line: &str, at: usize, name: &str) -> bool {
        let before = line[..at].ends_with(Self::NAME_STARTS);
        let after = line[at + name.len()..].starts_with(Self::NAME_ENDS);
        [before, after] == [true, true]
    }
}

#[cfg(test)]
mod tests {
    use super::MemberLine;

    #[test]
    fn a_member_is_renamed_and_its_signature_kept() {
        let line = "    member this.find(sku: BehaviorJSON) : Task<BehaviorJSON> =";
        assert_eq!(
            MemberLine::renamed(line, "find").as_deref(),
            Some("    member this.__tachyon_find(sku: BehaviorJSON) : Task<BehaviorJSON> =")
        );
        assert_eq!(
            MemberLine::signature(line, "find").as_deref(),
            Some("(sku: BehaviorJSON) : Task<BehaviorJSON>")
        );
        assert!(MemberLine::renamed(line, "sku").is_none());
    }

    #[test]
    fn an_abstract_member_becomes_a_member_with_a_body() {
        let line = "    abstract excluding: status: BehaviorJSON *                  page: BehaviorJSON -> Task<BehaviorJSON>";
        assert_eq!(
            MemberLine::concrete(line, "body").as_deref(),
            Some(
                "    member this.excluding(status: BehaviorJSON, page: BehaviorJSON) : Task<BehaviorJSON> = body"
            )
        );
        let line = "    abstract member count: unit -> Task<BehaviorJSON>";
        assert_eq!(
            MemberLine::concrete(line, "body").as_deref(),
            Some("    member this.count() : Task<BehaviorJSON> = body")
        );
        let line = "    abstract toSummary: BehaviorJSON -> BehaviorJSON";
        assert_eq!(
            MemberLine::concrete(line, "body").as_deref(),
            Some("    member this.toSummary(value0: BehaviorJSON) : BehaviorJSON = body")
        );
    }
}
