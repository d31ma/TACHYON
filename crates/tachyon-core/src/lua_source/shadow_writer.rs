use super::{LuaBody, LuaLine, LuaShadow, ShadowLines, WriterMode};

type Problems = Vec<(usize, &'static str)>;

pub(super) struct ShadowWriter {
    lines: ShadowLines,
    problems: Problems,
    fields: Vec<(String, String, usize)>,
    field_markers: Vec<(String, usize)>,
    pending: Vec<(String, usize)>,
    class: Option<String>,
    mode: WriterMode,
    joined: bool,
}

impl ShadowWriter {
    const MEMBER: &'static str = "  ";
    const STEPS: [fn(&mut Self, usize, &str, &str); 4] =
        [Self::top, Self::table, Self::parameter, Self::body];
    const AFTER_TABLE: [WriterMode; 2] = [WriterMode::Top, WriterMode::Table];
    const AFTER_SIGNATURE: [WriterMode; 2] = [WriterMode::Parameters, WriterMode::Body];
    const AFTER_BODY_LINE: [WriterMode; 2] = [WriterMode::Body, WriterMode::Top];
    const RECEIVERS: [&'static str; 2] = ["", "self."];
    const END: &'static str = "end";

    pub(super) fn write(source: &str, sanitized: &str, joined: bool) -> (ShadowLines, Problems) {
        let mut writer = Self {
            joined,
            lines: ShadowLines::default(),
            problems: Vec::new(),
            fields: Vec::new(),
            field_markers: Vec::new(),
            pending: Vec::new(),
            class: None,
            mode: WriterMode::Top,
        };
        let last = writer.each_line(source, sanitized);
        writer.flush_pending();
        writer.close_class(last);
        (writer.lines, writer.problems)
    }

    fn each_line(&mut self, source: &str, sanitized: &str) -> usize {
        let mut last = 0;
        for (index, (line, clean)) in source.lines().zip(sanitized.lines()).enumerate() {
            Self::STEPS[self.mode as usize](self, index, line, clean);
            last = index;
        }
        last
    }

    fn chosen(choices: [String; 2], second: bool) -> String {
        choices
            .into_iter()
            .nth(usize::from(second))
            .unwrap_or_default()
    }

    fn indent(&self) -> &'static str {
        ["", Self::MEMBER][usize::from(self.class.is_some())]
    }

    fn top(&mut self, index: usize, line: &str, _clean: &str) {
        match LuaLine::read(line) {
            LuaLine::Blank => self.pending.push((String::new(), index)),
            LuaLine::IndexSelf | LuaLine::ClassDoc => {}
            LuaLine::FieldDoc { name, yard } => self.fields.push((name, yard, index)),
            LuaLine::Doc(text) | LuaLine::Marker(text) => self.pending.push((text, index)),
            LuaLine::Comment => self.pending.push((String::from("#"), index)),
            LuaLine::ClassTable { name, base, open } => {
                self.open_class(index, name, base.as_deref(), open);
            }
            LuaLine::Constant { class, text } => self.constant(index, &class, &text),
            LuaLine::Method {
                class,
                name,
                is_static,
                rest,
            } => {
                self.method(
                    index,
                    &class,
                    &format!("{}{name}", Self::RECEIVERS[usize::from(is_static)]),
                    &rest,
                );
            }
            LuaLine::TableEnd | LuaLine::Other => {
                self.refuse(index, LuaShadow::statement_problem(line))
            }
        }
    }

    fn flush_pending(&mut self) {
        let indent = self.indent();
        for (text, index) in std::mem::take(&mut self.pending) {
            let line = Self::chosen([format!("{indent}{text}"), String::new()], text.is_empty());
            self.lines.push(line, index);
        }
    }

    fn refuse(&mut self, index: usize, message: &'static str) {
        self.flush_pending();
        self.problems.push((index, message));
        self.lines.push(format!("{}#", self.indent()), index);
    }

    fn check_owner(&mut self, index: usize, class: &str) {
        if self.class.as_deref() != Some(class) {
            self.problems.push((index, LuaShadow::FOREIGN_METHOD));
        }
    }

    fn open_class(&mut self, index: usize, name: String, base: Option<&str>, open: bool) {
        self.note_second_class(index);
        self.close_class(index);
        self.flush_pending();
        let header = base.map_or_else(
            || format!("class {name}"),
            |base| format!("class {name} < {base}"),
        );
        self.lines.push(header, index);
        self.class = Some(name);
        self.push_fields();
        self.mode = Self::AFTER_TABLE[usize::from(open)];
    }

    fn note_second_class(&mut self, index: usize) {
        if [self.class.is_some(), !self.joined] == [true, true] {
            self.problems.push((index, LuaShadow::SECOND_TYPE));
        }
    }

    fn close_class(&mut self, index: usize) {
        if self.class.take().is_some() {
            self.lines.push(Self::END, index);
        }
    }

    fn push_fields(&mut self) {
        for (field, yard, line) in std::mem::take(&mut self.fields) {
            self.lines
                .push(format!("{}# @return [{yard}]", Self::MEMBER), line);
            self.lines
                .push(format!("{}attr_reader :{field}", Self::MEMBER), line);
        }
    }

    fn constant(&mut self, index: usize, class: &str, text: &str) {
        self.flush_pending();
        self.check_owner(index, class);
        self.lines.push(format!("{}{text}", Self::MEMBER), index);
    }

    fn table(&mut self, index: usize, line: &str, _clean: &str) {
        match LuaLine::read(line.trim_start()) {
            LuaLine::Blank => self.lines.push("", index),
            LuaLine::Doc(text) => self.lines.push(format!("{}{text}", Self::MEMBER), index),
            LuaLine::Marker(text) => self
                .field_markers
                .push((format!("{}{text}", Self::MEMBER), index)),
            LuaLine::Comment => self.lines.push(format!("{}#", Self::MEMBER), index),
            LuaLine::TableEnd => self.end_table(index),
            _ => self.table_field(index, line.trim()),
        }
    }

    fn flush_field_markers(&mut self) {
        for (text, index) in std::mem::take(&mut self.field_markers) {
            self.lines.push(text, index);
        }
    }

    fn end_table(&mut self, index: usize) {
        self.flush_field_markers();
        self.lines.push("", index);
        self.mode = WriterMode::Top;
    }

    fn table_field(&mut self, index: usize, text: &str) {
        self.flush_field_markers();
        let field = text.strip_suffix(',').unwrap_or(text);
        let written = field
            .split_once('=')
            .and_then(|(name, value)| Self::field_text(name.trim(), value.trim()));
        self.push_or_refuse(index, written);
    }

    fn field_text(name: &str, value: &str) -> Option<String> {
        LuaLine::is_name(name).then(|| Self::member_line(name, value))
    }

    fn member_line(name: &str, value: &str) -> String {
        let constant = name.bytes().all(|byte| !byte.is_ascii_lowercase());
        Self::chosen(
            [
                format!("{}attr_reader :{name}", Self::MEMBER),
                format!("{}{name} = {value}", Self::MEMBER),
            ],
            constant,
        )
    }

    fn push_or_refuse(&mut self, index: usize, written: Option<String>) {
        match written {
            Some(text) => self.lines.push(text, index),
            None => self.refuse(index, LuaShadow::TABLE_FIELD),
        }
    }

    fn method(&mut self, index: usize, class: &str, name: &str, rest: &str) {
        self.flush_pending();
        self.check_owner(index, class);
        let closed = rest.split_once(')');
        let signature = closed.map_or_else(
            || format!("{}def {name}{}", Self::MEMBER, rest.trim_end()),
            |(parameters, _)| Self::closed_signature(name, parameters),
        );
        self.note_one_line(index, closed.map_or("", |(_, after)| after));
        self.lines.push(signature, index);
        self.mode = Self::AFTER_SIGNATURE[usize::from(closed.is_some())];
    }

    fn closed_signature(name: &str, parameters: &str) -> String {
        let parameters = parameters.trim_start_matches('(').trim();
        Self::chosen(
            [
                format!("{}def {name}({parameters})", Self::MEMBER),
                format!("{}def {name}", Self::MEMBER),
            ],
            parameters.is_empty(),
        )
    }

    fn note_one_line(&mut self, index: usize, after: &str) {
        if !after.trim().is_empty() {
            self.problems.push((index, LuaShadow::ONE_LINE));
        }
    }

    fn parameter(&mut self, index: usize, line: &str, _clean: &str) {
        let text = line.trim();
        let closing = text.starts_with(')');
        let converted = format!("{0}{0}{1}", Self::MEMBER, Self::parameter_text(text));
        let closing_line = format!("{}{text}", Self::MEMBER);
        self.lines
            .push(Self::chosen([converted, closing_line], closing), index);
        self.mode = Self::AFTER_SIGNATURE[usize::from(closing)];
    }

    fn parameter_text(text: &str) -> String {
        match LuaLine::read(text) {
            LuaLine::Marker(marker) => marker,
            LuaLine::Comment => String::from("#"),
            _ => text.to_owned(),
        }
    }

    fn body(&mut self, index: usize, line: &str, clean: &str) {
        let ends = line.trim_end() == Self::END;
        self.note_unindented(index, line, ends);
        let written = Self::chosen(
            [
                format!("{}{}", Self::MEMBER, LuaBody::translated(clean)),
                format!("{}{}", Self::MEMBER, Self::END),
            ],
            ends,
        );
        self.lines.push(written, index);
        self.mode = Self::AFTER_BODY_LINE[usize::from(ends)];
    }

    fn note_unindented(&mut self, index: usize, line: &str, ends: bool) {
        let unindented = [!ends, !line.is_empty(), !line.starts_with([' ', '\t'])];
        if unindented == [true, true, true] {
            self.problems.push((index, LuaShadow::ONE_LINE));
        }
    }
}
