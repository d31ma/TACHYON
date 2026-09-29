#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TypeForm {
    Class,
    Record,
    Enum,
    Union,
    Interface,
    Alias,
}

impl TypeForm {
    const DATA_CASE: &'static str = " of ";
    const ATTRIBUTE: &'static str = "[<";

    pub(crate) fn of(after_equals: &str, body: &[&str]) -> Self {
        let first = body
            .iter()
            .copied()
            .find(|line| !line.starts_with(Self::ATTRIBUTE))
            .unwrap_or("");
        let written = after_equals.trim();
        let shown = Some(written)
            .filter(|text| !text.is_empty())
            .unwrap_or(first);
        Self::from_first_text(shown, written.is_empty(), body)
    }

    fn from_first_text(shown: &str, from_body: bool, body: &[&str]) -> Self {
        if shown.starts_with('{') {
            Self::Record
        } else if shown.starts_with('|') || shown.contains(" | ") {
            Self::cases(shown, body)
        } else if shown.starts_with("abstract ") || shown.starts_with("interface") {
            Self::Interface
        } else if from_body || shown.starts_with("class") {
            Self::Class
        } else {
            Self::Alias
        }
    }

    fn cases(shown: &str, body: &[&str]) -> Self {
        let data = std::iter::once(&shown)
            .chain(body)
            .any(|line| line.contains(Self::DATA_CASE));
        Self::enum_or_union(data)
    }

    fn enum_or_union(data: bool) -> Self {
        if data {
            return Self::Union;
        }
        Self::Enum
    }
}

#[cfg(test)]
mod tests {
    use super::TypeForm;

    #[test]
    fn the_form_comes_from_the_text_after_equals_or_the_first_body_line() {
        assert_eq!(TypeForm::of("{ Name: string }", &[]), TypeForm::Record);
        assert_eq!(
            TypeForm::of("", &["{ Name: string", "Price: float }"]),
            TypeForm::Record
        );
        assert_eq!(TypeForm::of("Active | Closed", &[]), TypeForm::Enum);
        assert_eq!(
            TypeForm::of("", &["| Circle of float", "| Square"]),
            TypeForm::Union
        );
        assert_eq!(
            TypeForm::of("", &["abstract member Find : int -> Order"]),
            TypeForm::Interface
        );
        assert_eq!(
            TypeForm::of(
                "",
                &[
                    "[<Convert({})>]",
                    "abstract toSummary: BehaviorJSON -> BehaviorJSON"
                ]
            ),
            TypeForm::Interface
        );
        assert_eq!(
            TypeForm::of("", &["member this.Count = 1"]),
            TypeForm::Class
        );
        assert_eq!(TypeForm::of("class end", &[]), TypeForm::Class);
        assert_eq!(TypeForm::of("int", &[]), TypeForm::Alias);
    }
}
