use super::{
    AttributeLine, BlockKind, FsharpBody, FsharpLines, LineKind, MemberHeader, OpenBlock,
    RecordFields, SourceLine, TypeForm, TypeHeader,
};

pub(crate) struct ShadowWriter<'a> {
    lines: Vec<SourceLine<'a>>,
    written: Vec<(String, usize)>,
    blank: Vec<usize>,
    blocks: Vec<OpenBlock>,
    attributes: Vec<String>,
}

impl<'a> ShadowWriter<'a> {
    const ROLES: [&'static str; 8] = [
        "Service",
        "Store",
        "Client",
        "Helper",
        "ErrorHandler",
        "RequestRoute",
        "SocketRoute",
        "IdentityProvider",
    ];

    const STORE: &'static str = "Store";
    const BARE_STORE: &'static str = "[Store]";

    pub(crate) fn write(text: &'a str, code: &'a [bool]) -> Vec<(String, usize)> {
        let mut writer = Self {
            lines: SourceLine::split(text, code),
            written: Vec::new(),
            blank: Vec::new(),
            blocks: Vec::new(),
            attributes: Vec::new(),
        };
        writer.write_lines();
        writer.close_blocks(0);
        writer.flush_blank();
        writer.written
    }

    fn write_lines(&mut self) {
        for index in 0..self.lines.len() {
            self.write_line(index);
        }
    }

    fn write_line(&mut self, index: usize) {
        let trimmed = self.lines[index].trimmed();
        if trimmed.is_empty() || FsharpLines::is_directive(trimmed) {
            self.blank.push(index);
            return;
        }
        self.write_code_line(index);
    }

    fn write_code_line(&mut self, index: usize) {
        self.close_blocks(self.lines[index].indent());
        self.flush_blank();
        let context = self.blocks.last().map(|block| block.kind);
        let kind = LineKind::of(self.lines[index].trimmed());
        self.translate(index, context, kind);
    }

    fn translate(&mut self, index: usize, context: Option<BlockKind>, kind: LineKind) {
        match (context, kind) {
            (Some(BlockKind::Member), _)
            | (None, LineKind::Other | LineKind::Let | LineKind::Do) => {
                self.write_body(index);
            }
            (Some(BlockKind::Value), _) => self.write_value_body(index),
            (Some(BlockKind::Skip), _) => self.write_empty(index),
            (Some(BlockKind::Type), LineKind::Attribute) => self.write_member_attributes(index),
            (_, LineKind::Attribute) => self.write_attributes(index),
            (None, LineKind::Open) => self.write_open(index),
            (_, LineKind::Type) => self.write_type(index),
            (_, LineKind::Exception) => self.write_exception(index),
            (Some(BlockKind::Type), LineKind::Member) => self.write_member(index),
            (Some(BlockKind::Type), LineKind::MemberValue) => self.write_member_value(index),
            (Some(BlockKind::Type), LineKind::Constructor) => self.write_constructor(index),
            (Some(BlockKind::Type), LineKind::Let) => self.write_binding(index),
            (Some(BlockKind::Type), LineKind::Do) => self.write_skipped(index),
            (Some(BlockKind::Type), LineKind::UnionCase) => self.write_case(index),
            (Some(BlockKind::Type), LineKind::RecordStart | LineKind::Other) => {
                self.write_record_fields(index);
            }
            _ => self.write_empty(index),
        }
    }

    fn close_blocks(&mut self, indent: usize) {
        while self
            .blocks
            .last()
            .is_some_and(|block| block.indent >= indent)
        {
            let closer = self
                .blocks
                .pop()
                .map(|block| block.closer)
                .unwrap_or_default();
            self.append_to_last(&closer);
        }
    }

    fn append_to_last(&mut self, closer: &str) {
        if let Some((last, _)) = self.written.last_mut() {
            last.push_str(closer);
        }
    }

    fn flush_blank(&mut self) {
        for index in std::mem::take(&mut self.blank) {
            self.written.push((String::new(), index));
        }
    }

    fn push(&mut self, index: usize, text: &str) {
        let indent = " ".repeat(self.lines[index].indent());
        self.written.push((format!("{indent}{text}"), index));
    }

    fn write_empty(&mut self, index: usize) {
        self.attributes.clear();
        self.written.push((String::new(), index));
    }

    fn write_body(&mut self, index: usize) {
        let line = &self.lines[index];
        let text = FsharpBody::csharp(line.text, line.code);
        self.written.push((text, index));
    }

    fn write_open(&mut self, index: usize) {
        let namespace = self.lines[index]
            .trimmed()
            .trim_start_matches("open ")
            .trim();
        let namespace = namespace.trim_start_matches("type ").trim();
        self.push(index, &format!("using {namespace};"));
    }

    fn write_attributes(&mut self, index: usize) {
        let attributes = AttributeLine::read(self.lines[index].trimmed());
        self.write_attribute_list(index, attributes);
    }

    fn write_member_attributes(&mut self, index: usize) {
        let attributes = AttributeLine::on_member(self.lines[index].trimmed());
        self.write_attribute_list(index, attributes);
    }

    fn write_attribute_list(&mut self, index: usize, attributes: Vec<AttributeLine>) {
        for attribute in &attributes {
            self.push(index, &attribute.csharp);
        }
        self.attributes.extend(
            attributes
                .into_iter()
                .filter(|attribute| !Self::is_store_with_options(attribute))
                .map(|attribute| attribute.name),
        );
    }

    fn is_store_with_options(attribute: &AttributeLine) -> bool {
        [
            attribute.name == Self::STORE,
            attribute.csharp == Self::BARE_STORE,
        ] == [true, false]
    }

    fn write_type(&mut self, index: usize) {
        let body = self.body_lines(index);
        let managed = self
            .attributes
            .iter()
            .any(|name| Self::ROLES.contains(&name.as_str()));
        let header = TypeHeader::read(self.lines[index].trimmed(), &body, managed);
        self.open_type(index, header);
    }

    fn write_exception(&mut self, index: usize) {
        let header = TypeHeader::exception(self.lines[index].trimmed());
        self.open_type(index, header);
    }

    fn open_type(&mut self, index: usize, header: TypeHeader) {
        for line in header.lines {
            self.push(index, &line);
        }
        let indent = self.lines[index].indent();
        let block = OpenBlock::declared_type(indent, header.form, &header.name);
        self.blocks
            .extend((header.form != TypeForm::Alias).then_some(block));
        self.attributes.clear();
    }

    fn body_lines(&self, index: usize) -> Vec<&'a str> {
        let indent = self.lines[index].indent();
        self.lines[index + 1..]
            .iter()
            .filter(|line| !line.trimmed().is_empty())
            .take_while(|line| line.indent() > indent)
            .map(SourceLine::trimmed)
            .collect()
    }

    fn write_member(&mut self, index: usize) {
        let line = &self.lines[index];
        let form = self.blocks.last().map(|block| block.form);
        let header = MemberHeader::member(line.trimmed(), line.trimmed_code()).owned_by(form);
        self.open_member(index, &header);
    }

    fn write_member_value(&mut self, index: usize) {
        let header = MemberHeader::value(self.lines[index].trimmed());
        self.open_block(index, &header, BlockKind::Value);
    }

    fn write_value_body(&mut self, index: usize) {
        let line = &self.lines[index];
        let end = MemberHeader::accessors_start(line.text, line.code);
        let text = FsharpBody::csharp(&line.text[..end], &line.code[..end]);
        self.written.push((text, index));
    }

    fn write_constructor(&mut self, index: usize) {
        let type_name = self
            .blocks
            .last()
            .map(|block| block.name.clone())
            .unwrap_or_default();
        let header = MemberHeader::constructor(self.lines[index].trimmed(), &type_name);
        self.open_member(index, &header);
    }

    fn write_binding(&mut self, index: usize) {
        let line = &self.lines[index];
        let header = MemberHeader::binding(line.trimmed(), line.trimmed_code());
        self.open_member(index, &header);
    }

    fn open_member(&mut self, index: usize, header: &MemberHeader) {
        self.open_block(index, header, BlockKind::Member);
    }

    fn open_block(&mut self, index: usize, header: &MemberHeader, kind: BlockKind) {
        self.push(index, &header.text);
        let indent = self.lines[index].indent();
        self.blocks
            .push(OpenBlock::body(indent, kind, &header.closer));
        self.attributes.clear();
    }

    fn write_skipped(&mut self, index: usize) {
        self.write_empty(index);
        let indent = self.lines[index].indent();
        self.blocks
            .push(OpenBlock::body(indent, BlockKind::Skip, ""));
    }

    fn write_case(&mut self, index: usize) {
        let form = self.blocks.last().map(|block| block.form);
        let enumerated = form == Some(TypeForm::Enum);
        let case = self.lines[index].trimmed().trim_start_matches('|').trim();
        self.push(index, &Self::case_text(enumerated, case));
    }

    fn case_text(enumerated: bool, case: &str) -> String {
        if enumerated {
            return format!("{case},");
        }
        String::new()
    }

    fn write_record_fields(&mut self, index: usize) {
        let trimmed = self.lines[index].trimmed();
        let in_fields = self.in_record_fields(trimmed);
        let closes = trimmed.contains('}');
        self.set_fields_open(Self::stays_open(in_fields, closes));
        self.push(index, &Self::fields_text(in_fields, trimmed));
    }

    fn stays_open(in_fields: bool, closes: bool) -> bool {
        in_fields && !closes
    }

    fn fields_text(in_fields: bool, trimmed: &str) -> String {
        if in_fields {
            return RecordFields::csharp(trimmed);
        }
        String::new()
    }

    fn in_record_fields(&self, trimmed: &str) -> bool {
        let block = self.blocks.last();
        let record = block.is_some_and(|block| block.form == TypeForm::Record);
        let open = [
            block.is_some_and(|block| block.fields_open),
            trimmed.starts_with('{'),
        ]
        .contains(&true);
        record && open
    }

    fn set_fields_open(&mut self, open: bool) {
        if let Some(block) = self.blocks.last_mut() {
            block.fields_open = open;
        }
    }
}
