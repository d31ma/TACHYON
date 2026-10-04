use super::CppAttribute;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CppMemberKind {
    Field,
    Method,
    Constructor,
    Destructor,
    Operator,
    NestedType,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CppParameter {
    pub(crate) name: String,
    pub(crate) type_text: String,
    pub(crate) markers: Vec<CppAttribute>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CppMember {
    pub(crate) kind: CppMemberKind,
    pub(crate) name: String,
    pub(crate) name_at: usize,
    pub(crate) type_text: String,
    pub(crate) parameters: Vec<CppParameter>,
    pub(crate) public: bool,
    pub(crate) is_static: bool,
    pub(crate) has_initializer: bool,
    pub(crate) declarators: usize,
    pub(crate) markers: Vec<CppAttribute>,
    pub(crate) terminator: usize,
}

impl CppMember {
    pub(crate) fn marked(&self, name: &str) -> bool {
        self.markers.iter().any(|marker| marker.name == name)
    }

    pub(crate) fn plain_type(&self) -> String {
        CppTypeText::plain(&self.type_text)
    }

    pub(crate) fn is_reference(&self) -> bool {
        self.type_text.contains('&') || self.type_text.contains('*')
    }
}

pub(crate) struct CppTypeText;

impl CppTypeText {
    const QUALIFIERS: [&'static str; 3] = ["const", "volatile", "typename"];

    pub(crate) fn plain(type_text: &str) -> String {
        let words: Vec<&str> = type_text
            .split_whitespace()
            .filter(|word| !Self::QUALIFIERS.contains(word))
            .collect();
        words
            .join(" ")
            .trim_end_matches(['&', '*', ' '])
            .trim()
            .to_owned()
    }
}
