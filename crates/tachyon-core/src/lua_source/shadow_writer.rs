use super::{LuaBody, LuaLine, LuaShadow, ShadowLines, WriterMode};

pub(super) struct ShadowWriter {
    lines: ShadowLines,
    problems: Vec<(usize, &'static str)>,
    fields: Vec<(String, String, usize)>,
    field_markers: Vec<(String, usize)>,
    pending: Vec<(String, usize)>,
    class: Option<String>,
    mode: WriterMode,
    joined: bool,
}

impl ShadowWriter {
    const MEMBER: &'static str = "  ";
    const BODY: &'static str = "  ";

    pub(super) fn write(
        source: &str,
        sanitized: &str,
        joined: bool,
    ) -> (ShadowLines, Vec<(usize, &'static str)>) {
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
        let mut last = 0;
        for (index, (line, clean)) in source.lines().zip(sanitized.lines()).enumerate() {
            writer.line(index, line, clean);
            last = index;
        }
        writer.flush_pending();
        if writer.class.is_some() {
            writer.lines.push("end", last);
        }
        (writer.lines, writer.problems)
    }

    fn line(&mut self, index: usize, line: &str, clean: &str) {
        match self.mode {
            WriterMode::Top => self.top(index, line),
            WriterMode::Table => self.table(index, line),
            WriterMode::Parameters => self.parameter(index, line),
            WriterMode::Body => self.body(index, line, clean),
        }
    }

    fn indent(&self) -> &'static str {
        if self.class.is_some() { Self::MEMBER } else { "" }
    }

    fn top(&mut self, index: usize, line: &str) {
        match LuaLine::read(line) {
            LuaLine::Blank => self.pending.push((String::new(), index)),
            LuaLine::IndexSelf | LuaLine::ClassDoc { .. } => {}
            LuaLine::FieldDoc { name, yard } => {
                self.fields.push((name, yard, index));
            }
            LuaLine::Doc(text) | LuaLine::Marker(text) => self.pending.push((text, index)),
            LuaLine::Comment => self.pending.push((String::from("#"), index)),
            LuaLine::ClassTable { name, base, open } => self.class_table(index, name, base, open),
            LuaLine::Constant { class, text } => {
                self.flush_pending();
                self.check_owner(index, &class, LuaShadow::FOREIGN_METHOD);
                self.lines.push(format!("{}{text}", Self::MEMBER), index);
            }
            LuaLine::Method { class, name, is_static, rest } => {
                self.flush_pending();
                self.method(index, &class, &name, is_static, &rest);
            }
            LuaLine::TableEnd | LuaLine::Other => {
                self.refuse(index, LuaShadow::statement_problem(line));
            }
        }
    }

    fn flush_pending(&mut self) {
        let indent = self.indent();
        for (text, index) in std::mem::take(&mut self.pending) {
            let line = if text.is_empty() { text } else { format!("{indent}{text}") };
            self.lines.push(line, index);
        }
    }

    fn refuse(&mut self, index: usize, message: &'static str) {
        self.flush_pending();
        self.problems.push((index, message));
        self.lines.push(format!("{}#", self.indent()), index);
    }

    fn check_owner(&mut self, index: usize, class: &str, message: &'static str) {
        if self.class.as_deref() != Some(class) {
            self.problems.push((index, message));
        }
    }

    fn class_table(&mut self, index: usize, name: String, base: Option<String>, open: bool) {
        if self.class.is_some() && !self.joined {
            self.refuse(index, LuaShadow::SECOND_TYPE);
            return;
        }
        if self.class.take().is_some() {
            self.lines.push("end", index);
        }
        self.flush_pending();
        let header = base.map_or_else(|| format!("class {name}"), |base| format!("class {name} < {base}"));
        self.lines.push(header, index);
        self.class = Some(name);
        for (field, yard, line) in std::mem::take(&mut self.fields) {
            self.lines.push(format!("{}# @return [{yard}]", Self::MEMBER), line);
            self.lines.push(format!("{}attr_reader :{field}", Self::MEMBER), line);
        }
        if open {
            self.mode = WriterMode::Table;
        }
    }

    fn table(&mut self, index: usize, line: &str) {
        let member = Self::MEMBER;
        match LuaLine::read(line.trim_start()) {
            LuaLine::Blank => self.lines.push("", index),
            LuaLine::Doc(text) => self.lines.push(format!("{member}{text}"), index),
            LuaLine::Marker(text) => self.field_markers.push((format!("{member}{text}"), index)),
            LuaLine::Comment => self.lines.push(format!("{member}#"), index),
            LuaLine::TableEnd => {
                self.flush_field_markers();
                self.lines.push("", index);
                self.mode = WriterMode::Top;
            }
            _ => {
                self.flush_field_markers();
                self.table_field(index, line.trim());
            }
        }
    }

    fn flush_field_markers(&mut self) {
        for (text, index) in std::mem::take(&mut self.field_markers) {
            self.lines.push(text, index);
        }
    }

    fn table_field(&mut self, index: usize, text: &str) {
        let field = text.strip_suffix(',').unwrap_or(text);
        let Some((name, value)) = field.split_once('=') else {
            self.refuse(index, LuaShadow::TABLE_FIELD);
            return;
        };
        let name = name.trim();
        if !LuaLine::is_name(name) {
            self.refuse(index, LuaShadow::TABLE_FIELD);
            return;
        }
        let constant = name.bytes().all(|byte| !byte.is_ascii_lowercase());
        let text = if constant {
            format!("{}{name} = {}", Self::MEMBER, value.trim())
        } else {
            format!("{}attr_reader :{name}", Self::MEMBER)
        };
        self.lines.push(text, index);
    }

    fn method(&mut self, index: usize, class: &str, name: &str, is_static: bool, rest: &str) {
        self.check_owner(index, class, LuaShadow::FOREIGN_METHOD);
        let receiver = if is_static { "self." } else { "" };
        let member = Self::MEMBER;
        let Some(close) = rest.find(')') else {
            self.lines.push(format!("{member}def {receiver}{name}{}", rest.trim_end()), index);
            self.mode = WriterMode::Parameters;
            return;
        };
        if !rest[close + 1..].trim().is_empty() {
            self.problems.push((index, LuaShadow::ONE_LINE));
        }
        let parameters = rest[1..close].trim();
        let signature = if parameters.is_empty() {
            format!("{member}def {receiver}{name}")
        } else {
            format!("{member}def {receiver}{name}({parameters})")
        };
        self.lines.push(signature, index);
        self.mode = WriterMode::Body;
    }

    fn parameter(&mut self, index: usize, line: &str) {
        let text = line.trim();
        let member = Self::MEMBER;
        if text.starts_with(')') {
            self.lines.push(format!("{member}{text}"), index);
            self.mode = WriterMode::Body;
            return;
        }
        let converted = match LuaLine::read(text) {
            LuaLine::Marker(marker) => marker,
            LuaLine::Comment => String::from("#"),
            _ => text.to_owned(),
        };
        self.lines.push(format!("{member}{member}{converted}"), index);
    }

    fn body(&mut self, index: usize, line: &str, clean: &str) {
        if line.trim_end() == "end" {
            self.lines.push(format!("{}end", Self::MEMBER), index);
            self.mode = WriterMode::Top;
            return;
        }
        if !line.is_empty() && !line.starts_with([' ', '\t']) {
            self.problems.push((index, LuaShadow::ONE_LINE));
        }
        let translated = LuaBody::translated(clean);
        self.lines.push(format!("{}{translated}", Self::BODY), index);
    }
}
