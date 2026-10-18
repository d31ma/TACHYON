pub(crate) struct LuaFields {
    type_name: String,
    marked: bool,
    found: Vec<(String, String, bool)>,
}

impl LuaFields {
    const TYPE: &'static str = "---@type";
    const NEEDS: &'static str = "-- @Needs";
    const EMPTY: &'static str = "nil";
    const TABLE_END: &'static str = "\n}";

    pub(crate) fn without_values(source: &str, class: &str) -> Vec<(String, String, bool)> {
        let body = Self::table_body(source, class).unwrap_or_default();
        let mut fields = Self {
            type_name: String::new(),
            marked: false,
            found: Vec::new(),
        };
        body.lines().for_each(|line| fields.read(line.trim()));
        fields.found
    }

    fn table_body<'s>(source: &'s str, class: &str) -> Option<&'s str> {
        let opening = format!("{class} = {{");
        let start = source.find(&opening)? + opening.len();
        let rest = &source[start..];
        let closed_at_once = rest.trim_start_matches([' ', '\t']).starts_with('}');
        rest.split(Self::TABLE_END)
            .next()
            .filter(|_| !closed_at_once)
    }

    fn read(&mut self, line: &str) {
        let steps: [fn(&mut Self, &str) -> bool; 3] = [Self::typed, Self::needs, Self::field];
        let _ = steps.iter().any(|step| step(self, line));
    }

    fn typed(&mut self, line: &str) -> bool {
        line.strip_prefix(Self::TYPE)
            .map(|rest| rest.trim().clone_into(&mut self.type_name))
            .is_some()
    }

    fn needs(&mut self, line: &str) -> bool {
        let marked = line.starts_with(Self::NEEDS);
        self.marked |= marked;
        marked
    }

    fn field(&mut self, line: &str) -> bool {
        let pair = line.trim_end_matches(',').split_once('=');
        let empty = pair.filter(|(_, value)| value.trim() == Self::EMPTY);
        self.found.extend(empty.map(|(name, _)| {
            (
                name.trim().to_owned(),
                std::mem::take(&mut self.type_name),
                self.marked,
            )
        }));
        self.type_name.clear();
        self.marked = false;
        pair.is_some()
    }
}
