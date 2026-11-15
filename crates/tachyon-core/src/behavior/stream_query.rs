use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StreamQuery {
    model: String,
}

impl StreamQuery {
    pub(crate) const PREFIX: &'static str = "/.tachyon/query/";
    const MOST_LENGTH: usize = 64;
    const FORM: &'static str = "query names the model whose events set the stream's parameters, such as \"StatsWindow\". A model name starts with a letter and has at most 64 letters, digits and underscores. Each time a service publishes that model's event, Tachyon fills the {name} parts of the path from its fields, sends the other fields after ?, and opens the stream again";
    const NEEDS_QUERY: &'static str = "A stream path with a {name} part needs a query model to fill it, for example {\"path\": \"/stats/{window}\", \"query\": \"StatsWindow\"}";
    const PATH_PART: &'static str = "A {name} part of a stream path starts with a letter or an underscore and has only letters, digits and underscores, such as {window}";

    pub(crate) fn read(value: &Value) -> Result<Self, String> {
        let model = value
            .as_str()
            .filter(|model| Self::is_model(model))
            .ok_or_else(|| String::from(Self::FORM))?;
        Ok(Self {
            model: model.to_owned(),
        })
    }

    pub(crate) fn addressed(&self, path: &str) -> String {
        format!("{}{}{path}", Self::PREFIX, self.model)
    }

    pub(crate) fn check_path(path: &str, has_query: bool) -> Result<(), String> {
        let parts = Self::path_parts(path)?;
        let unfilled = [parts > 0, has_query] == [true, false];
        (!unfilled)
            .then_some(())
            .ok_or_else(|| String::from(Self::NEEDS_QUERY))
    }

    fn path_parts(path: &str) -> Result<usize, String> {
        let pieces: Vec<&str> = path.split('{').skip(1).collect();
        let well_formed = pieces.iter().all(|piece| Self::is_path_part(piece));
        let stray_close = path.split('{').next().unwrap_or("").contains('}');
        [well_formed, stray_close]
            .eq(&[true, false])
            .then_some(pieces.len())
            .ok_or_else(|| String::from(Self::PATH_PART))
    }

    fn is_path_part(piece: &str) -> bool {
        let (name, after) = piece.split_once('}').unwrap_or(("", "}"));
        let starts_well = name.starts_with(Self::is_word_start);
        let plain = name.bytes().all(Self::is_word_byte);
        [starts_well, plain, after.contains('}')] == [true, true, false]
    }

    fn is_model(model: &str) -> bool {
        let starts_with_letter = model.starts_with(|letter: char| letter.is_ascii_alphabetic());
        let short = model.len() <= Self::MOST_LENGTH;
        let plain = model.bytes().all(Self::is_word_byte);
        [starts_with_letter, short, plain] == [true, true, true]
    }

    fn is_word_start(letter: char) -> bool {
        [letter.is_ascii_alphabetic(), letter == '_'].contains(&true)
    }

    fn is_word_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }
}
