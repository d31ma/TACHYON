use super::{Declaration, Failure, Input, Path, Role};

pub(super) mod cohesion;
pub(super) mod design_check;
pub(super) mod duplication;
mod go;
mod load_time;
mod ruby;
pub(super) mod rule_switch;
pub(super) mod simpler_forms;
mod tokens;
pub(super) mod utility_layout;
use tokens::Tokens;

pub(super) fn has_framework_annotation(path: &Path, source: &str) -> Result<bool, Failure> {
    let input = Input::new(path, source);
    if Tokens::read(&input)?.has_framework_marker() {
        return Ok(true);
    }
    if input.extension != "py" {
        return Ok(false);
    }
    let code = input.masked_code()?;
    let annotated = crate::python_parameters::PythonAnnotated::markers(&code, source, |name| {
        crate::behavior::is_framework_marker(name) || crate::behavior::is_parameter_marker(name)
    });
    Ok(!annotated.markers.is_empty())
}

pub(super) fn has_role(path: &Path, source: &str, marker: &str) -> Result<bool, Failure> {
    let input = Input::new(path, source);
    let tokens = Tokens::read(&input)?;
    if !tokens.has_role_marker(marker) {
        return Ok(false);
    }
    tokens.check_complete(&input)?;
    let mut parser = Parser::new(&input, &tokens);
    parser.read()?;
    let matches = parser.primary.is_some_and(|role| role.marker == marker);
    parser.finish(None)?;
    Ok(matches)
}

pub(super) fn validate_error_source(path: &Path, source: &str) -> Result<(), Failure> {
    let input = Input::new(path, source);
    if path.parent() != Some(Path::new("server/errors"))
        || !crate::convention::is_yon_language(path)
    {
        return Err(input.failure(
            0,
            "Application error types belong directly in server/errors in a Yon language",
            "Move this native error type to server/errors/<Type>.<ext>; Tac error handling is unchanged.",
        ));
    }
    let tokens = Tokens::read(&input)?;
    tokens.check_complete(&input)?;
    if tokens.has_framework_marker() {
        return Err(input.failure(
            0,
            "Application error types are plain native types, not annotated providers",
            "Keep the error declaration here without Tachyon annotations. Put @ErrorHandler on its handler in server/controllers.",
        ));
    }
    let mut parser = Parser::new(&input, &tokens);
    parser.read()?;
    parser.finish(path.file_stem().and_then(|name| name.to_str()))?;
    Ok(())
}

pub(super) fn has_test_annotation(path: &Path, source: &str) -> Result<bool, Failure> {
    let input = Input::new(path, source);
    Ok(Tokens::read(&input)?.has_test_marker())
}

pub(super) fn validate(path: &Path, source: &str) -> Result<bool, Failure> {
    let input = Input::new(path, source);
    let tokens = Tokens::read(&input)?;
    let handler = handler_name(path);
    if handler.is_none() && !tokens.has_framework_marker() {
        return Ok(false);
    }
    tokens.check_complete(&input)?;
    let mut parser = Parser::new(&input, &tokens);
    parser.read()?;
    parser.finish(handler)
}

fn handler_name(path: &Path) -> Option<&'static str> {
    let portable = path.to_string_lossy().replace('\\', "/");
    let stem = path.file_stem()?.to_str()?;
    if stem == "middleware" && !portable.contains('/') {
        return Some("Middleware");
    }
    if portable.starts_with("server/workers/") {
        return Some("Handler");
    }
    None
}

struct Parser<'a, 'b> {
    input: &'a Input<'b>,
    tokens: &'a Tokens<'b>,
    at: usize,
    declared: Option<String>,
    implementations: Vec<(String, usize)>,
    primary: Option<Role>,
    header_items: usize,
    namespace_end: Option<usize>,
}

impl<'a, 'b> Parser<'a, 'b> {
    fn new(input: &'a Input<'b>, tokens: &'a Tokens<'b>) -> Self {
        Self {
            input,
            tokens,
            at: 0,
            declared: None,
            implementations: Vec::new(),
            primary: None,
            header_items: 0,
            namespace_end: None,
        }
    }

    fn read(&mut self) -> Result<(), Failure> {
        self.skip_php_tag();
        self.skip_go_package()?;
        self.skip_shebang_line();
        if self.skip_first_string() {
            self.header_items += 1;
        }
        while self.at < self.tokens.items.len() {
            if self.namespace_end == Some(self.at) {
                self.namespace_end = None;
                self.at += 1;
                continue;
            }
            if self.is_import() {
                self.read_import()?;
                self.count_import();
                continue;
            }
            if self.read_header_statement()? {
                self.header_items += 1;
                continue;
            }
            self.read_type()?;
        }
        Ok(())
    }

    fn count_import(&mut self) {
        if !matches!(self.input.extension, "java" | "groovy") {
            self.header_items += 1;
        }
    }

    fn skip_php_tag(&mut self) {
        if self.input.extension == "php"
            && self.word(0) == "<"
            && self.word(1) == "?"
            && self.word(2) == "php"
        {
            self.at = 3;
        }
    }

    fn skip_shebang_line(&mut self) {
        if !matches!(self.input.extension, "js" | "ts") || !self.input.source.starts_with("#!") {
            return;
        }
        let line_end = self
            .input
            .source
            .find('\n')
            .unwrap_or(self.input.source.len());
        while self
            .tokens
            .items
            .get(self.at)
            .is_some_and(|token| token.start < line_end)
        {
            self.at += 1;
        }
    }

    fn skip_first_string(&mut self) -> bool {
        let Some(token) = self.tokens.items.get(self.at) else {
            return false;
        };
        let allowed = match self.input.extension {
            "py" => token.literal,
            "js" | "ts" => matches!(token.text, "\"use strict\"" | "'use strict'"),
            _ => false,
        };
        if !allowed {
            return false;
        }
        let next = self.at + 1;
        if self.input.extension != "py" && self.word(next) == ";" {
            self.at = next + 1;
            return true;
        }
        if !self.tokens.new_line_before(next) {
            return false;
        }
        self.at = next;
        true
    }

    fn read_header_statement(&mut self) -> Result<bool, Failure> {
        match self.input.extension {
            "py" if self.starts_type_checking_block() => self.read_type_checking_block()?,
            "php" if self.word(self.at) == "declare" => self.read_php_declare()?,
            "php" if self.is_php_closing_tag() => self.at += 2,
            "dart" if self.word(self.at) == "library" => self.read_dart_library()?,
            "rs" if self.starts_rust_inner_attribute() => self.read_rust_inner_attribute()?,
            "cs" if self.word(self.at) == "namespace" => self.read_csharp_namespace()?,
            "cs" if self.starts_csharp_nullable() => self.read_csharp_nullable()?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn starts_type_checking_block(&self) -> bool {
        self.word(self.at) == "if"
            && self.word(self.at + 1) == "TYPE_CHECKING"
            && self.word(self.at + 2) == ":"
    }

    fn read_type_checking_block(&mut self) -> Result<(), Failure> {
        self.at += 3;
        while self.at < self.tokens.items.len() && !self.starts_python_top_level_line(self.at) {
            if !self.is_import() {
                return Err(self.refused(
                    self.at,
                    "code other than imports inside 'if TYPE_CHECKING:'",
                ));
            }
            self.read_import()?;
        }
        Ok(())
    }

    fn starts_python_top_level_line(&self, at: usize) -> bool {
        let continues_line = at > 0 && self.word(at - 1) == "\\";
        self.tokens.new_line_before(at) && self.tokens.column(at) == 0 && !continues_line
    }

    fn read_php_declare(&mut self) -> Result<(), Failure> {
        let start = self.at;
        self.at += 1;
        if self.word(self.at) != "(" {
            return Err(self.refused(start, "an invalid declare statement"));
        }
        self.at = self
            .tokens
            .after_group(self.at)
            .ok_or_else(|| self.refused(start, "an unclosed declare statement"))?;
        if self.word(self.at) != ";" {
            return Err(self.refused(self.at, "code after a declare statement"));
        }
        self.at += 1;
        Ok(())
    }

    fn is_php_closing_tag(&self) -> bool {
        self.word(self.at) == "?"
            && self.word(self.at + 1) == ">"
            && self.at + 2 == self.tokens.items.len()
    }

    fn read_dart_library(&mut self) -> Result<(), Failure> {
        let start = self.at;
        self.at += 1;
        while self.is_name(self.at) || self.word(self.at) == "." {
            self.at += 1;
        }
        if self.word(self.at) != ";" {
            return Err(self.refused(start, "an invalid library directive"));
        }
        self.at += 1;
        Ok(())
    }

    fn starts_rust_inner_attribute(&self) -> bool {
        self.word(self.at) == "#" && self.word(self.at + 1) == "!" && self.word(self.at + 2) == "["
    }

    fn read_rust_inner_attribute(&mut self) -> Result<(), Failure> {
        let start = self.at;
        self.at = self
            .tokens
            .after_group(self.at + 2)
            .ok_or_else(|| self.refused(start, "an unclosed attribute"))?;
        Ok(())
    }

    fn read_csharp_namespace(&mut self) -> Result<(), Failure> {
        let start = self.at;
        self.at += 1;
        while self.is_name(self.at) || self.word(self.at) == "." {
            self.at += 1;
        }
        if self.word(self.at) == ";" {
            self.at += 1;
            return Ok(());
        }
        if self.word(self.at) != "{" || self.namespace_end.is_some() {
            return Err(self.refused(start, "an invalid namespace declaration"));
        }
        let end = self
            .tokens
            .after_group(self.at)
            .ok_or_else(|| self.refused(start, "an unclosed namespace"))?;
        self.namespace_end = Some(end - 1);
        self.at += 1;
        Ok(())
    }

    fn starts_csharp_nullable(&self) -> bool {
        self.word(self.at) == "#" && self.word(self.at + 1) == "nullable"
    }

    fn read_csharp_nullable(&mut self) -> Result<(), Failure> {
        let start = self.at;
        self.at += 2;
        if !matches!(self.word(self.at), "enable" | "disable" | "restore")
            || self.tokens.new_line_before(self.at)
        {
            return Err(self.refused(start, "an invalid #nullable line"));
        }
        self.at += 1;
        if matches!(self.word(self.at), "warnings" | "annotations")
            && !self.tokens.new_line_before(self.at)
        {
            self.at += 1;
        }
        if !self.tokens.new_line_before(self.at) {
            return Err(self.refused(self.at, "code after a #nullable line"));
        }
        Ok(())
    }

    fn is_name(&self, at: usize) -> bool {
        self.tokens
            .items
            .get(at)
            .is_some_and(tokens::Token::is_name)
    }

    fn word(&self, at: usize) -> &str {
        self.tokens.items.get(at).map_or("", |token| token.text)
    }

    fn position(&self, at: usize) -> usize {
        self.tokens
            .items
            .get(at)
            .map_or(self.input.source.len(), |token| token.start)
    }

    fn refused(&self, at: usize, item: &str) -> Failure {
        let realm = if self.input.parent_folder().starts_with("client") {
            "client"
        } else {
            "server"
        };
        self.input.failure(
            self.position(at),
            format!("An annotated or handler file holds one class and its imports; found {item}"),
            format!("Move another annotated class to its own named file in its role folder. Move data-only types to '{realm}/models/' and plain helper code to '{realm}/shared/', or make it a service method. Keep TestSetup in its own named file beside the test unit."),
        )
    }

    fn is_import(&self) -> bool {
        let word = self.word(self.at);
        match self.input.extension {
            "js" | "ts" | "dart" | "swift" | "go" => word == "import",
            "kt" => matches!(word, "import" | "package"),
            "py" => matches!(word, "import" | "from"),
            "rs" => self.rust_use_start().is_some(),
            "php" => word == "use",
            "rb" => matches!(word, "require" | "require_relative"),
            "java" | "groovy" => matches!(word, "package" | "import"),
            "cs" => word == "using" || (word == "global" && self.word(self.at + 1) == "using"),
            _ => false,
        }
    }

    fn rust_use_start(&self) -> Option<usize> {
        let mut at = self.at;
        if self.word(at) == "pub" {
            at += 1;
            if self.word(at) == "(" {
                at = self.tokens.after_group(at)?;
            }
        }
        if self.word(at) == "use" {
            return Some(at);
        }
        None
    }

    fn read_import(&mut self) -> Result<(), Failure> {
        if matches!(self.input.extension, "js" | "ts" | "dart") {
            return self.read_string_import();
        }
        if self.input.extension == "go" {
            return self.read_go_import();
        }
        if self.input.extension == "rb" {
            return self.read_ruby_import();
        }
        if self.input.extension == "rs"
            && let Some(at) = self.rust_use_start()
        {
            self.at = at;
        }
        let start = self.at;
        let semicolon = matches!(self.input.extension, "rs" | "php" | "java" | "cs");
        self.at += 1;
        if self.names_imported_kind() {
            self.at += 1;
        }
        while self.at < self.tokens.items.len() {
            if self.word(self.at) == ";" {
                self.at += 1;
                return Ok(());
            }
            if !semicolon && self.tokens.new_line_before(self.at) && self.word(self.at - 1) != "\\"
            {
                return Ok(());
            }
            if self.import_group_allowed() {
                self.at = self
                    .tokens
                    .after_group(self.at)
                    .ok_or_else(|| self.refused(start, "an unclosed import"))?;
                continue;
            }
            if !self.import_token_allowed() {
                return Err(self.refused(self.at, "code outside an import or class"));
            }
            self.at += 1;
        }
        if semicolon {
            return Err(self.refused(start, "an unterminated import"));
        }
        Ok(())
    }

    fn names_imported_kind(&self) -> bool {
        let word = self.word(self.at);
        match self.input.extension {
            "php" => matches!(word, "function" | "const"),
            "swift" => matches!(
                word,
                "typealias" | "struct" | "class" | "enum" | "protocol" | "let" | "var" | "func"
            ),
            _ => false,
        }
    }

    fn import_group_allowed(&self) -> bool {
        match self.input.extension {
            "rs" | "php" => self.word(self.at) == "{",
            "py" => self.word(self.at) == "(",
            _ => false,
        }
    }

    fn import_token_allowed(&self) -> bool {
        let token = &self.tokens.items[self.at];
        if token.is_name() {
            return !matches!(
                token.text,
                "class" | "struct" | "fn" | "fun" | "function" | "const" | "let" | "var" | "return"
            );
        }
        if self.input.extension == "cs" && token.text == "=" {
            return true;
        }
        matches!(token.text, "." | ":" | "*" | "," | "\\")
    }

    fn read_string_import(&mut self) -> Result<(), Failure> {
        let start = self.at;
        self.at += 1;
        while self.at < self.tokens.items.len() && !self.tokens.items[self.at].literal {
            if self.word(self.at) == "{" {
                self.at = self
                    .tokens
                    .after_group(self.at)
                    .ok_or_else(|| self.refused(start, "an unclosed import"))?;
                continue;
            }
            if !self.tokens.items[self.at].is_name() && !matches!(self.word(self.at), "*" | ",") {
                return Err(self.refused(self.at, "an executable import expression"));
            }
            self.at += 1;
        }
        if self.at == self.tokens.items.len() {
            return Err(self.refused(start, "an import without a module name"));
        }
        self.at += 1;
        if matches!(self.word(self.at), "with" | "assert") {
            self.at += 1;
            self.at = self
                .tokens
                .after_group(self.at)
                .ok_or_else(|| self.refused(start, "invalid import attributes"))?;
        }
        if self.input.extension == "dart" {
            while self.at < self.tokens.items.len() && self.word(self.at) != ";" {
                if !self.tokens.items[self.at].is_name() && self.word(self.at) != "," {
                    return Err(self.refused(self.at, "code after an import"));
                }
                self.at += 1;
            }
        }
        if self.word(self.at) == ";" {
            self.at += 1;
            return Ok(());
        }
        if self.at == self.tokens.items.len() || self.tokens.new_line_before(self.at) {
            return Ok(());
        }
        Err(self.refused(self.at, "code after an import"))
    }

    fn read_type(&mut self) -> Result<(), Failure> {
        let start = self.at;
        let markers = self.read_markers()?;
        if self.input.extension == "go" {
            return self.read_go_declaration(start, &markers);
        }
        self.skip_modifiers()?;
        if self.at == self.tokens.items.len() {
            return Err(self.refused(start, "an annotation or modifier with no class after it"));
        }
        let kind = self.word(self.at).to_owned();
        if kind == "impl" && self.input.extension == "rs" {
            return self.read_implementation(start, &markers);
        }
        if !self.allowed_type(&kind) {
            return Err(self.refused_top_level());
        }
        self.at += 1;
        if kind == "record" && matches!(self.word(self.at), "class" | "struct") {
            self.at += 1;
        }
        let name = self.read_name()?;
        if self.declared.is_some() {
            return Err(self.refused(start, &Self::another_type(&kind, &name, &markers)));
        }
        self.check_roles(&markers, &name)?;
        self.declared = Some(name);
        self.read_type_body(start)
    }

    fn refused_top_level(&self) -> Failure {
        let item = self.tokens.items.get(self.at);
        let found = if item.is_some_and(|token| token.literal) {
            String::from("a top-level literal or expression")
        } else {
            let word: String = self.word(self.at).chars().take(128).collect();
            format!("top-level '{word}'")
        };
        self.refused(self.at, &found)
    }

    fn another_type(kind: &str, name: &str, markers: &[(String, usize)]) -> String {
        if markers.is_empty() {
            return format!("another {kind} '{name}'");
        }
        format!("another annotated {kind} '{name}', which belongs in its own file")
    }

    fn read_markers(&mut self) -> Result<Vec<(String, usize)>, Failure> {
        let mut markers = Vec::new();
        while self.tokens.marker_start(self.at) {
            let start = self.at;
            if self.word(self.at) == "@" {
                self.at += 1;
                markers.push(self.read_marker(start)?);
            } else {
                self.read_attribute_group(start, &mut markers)?;
            }
        }
        Ok(markers)
    }

    fn read_marker(&mut self, start: usize) -> Result<(String, usize), Failure> {
        let name = self.read_marker_path();
        if name.is_empty() {
            return Err(self.refused(start, "an annotation without a name"));
        }
        if self.word(self.at) == "(" {
            let open = self.at;
            self.at = self
                .tokens
                .after_group(self.at)
                .ok_or_else(|| self.refused(start, "an unclosed annotation"))?;
            self.check_marker_arguments(open, self.at)?;
        }
        Ok((name, self.position(start)))
    }

    fn read_attribute_group(
        &mut self,
        start: usize,
        markers: &mut Vec<(String, usize)>,
    ) -> Result<(), Failure> {
        if self.word(self.at) == "#" {
            self.at += 1;
        }
        let end = self
            .tokens
            .after_group(self.at)
            .ok_or_else(|| self.refused(start, "an unclosed attribute"))?;
        self.at += 1;
        while self.at + 1 < end {
            markers.push(self.read_marker(start)?);
            if self.input.extension == "rs" && self.word(self.at) == "=" && self.at + 2 < end {
                self.at = end - 1;
            }
            if self.word(self.at) != "," {
                break;
            }
            self.at += 1;
        }
        if self.at + 1 != end {
            return Err(self.refused(self.at, "unsupported attribute syntax"));
        }
        self.at = end;
        Ok(())
    }

    fn read_marker_path(&mut self) -> String {
        let start = self.at;
        if self
            .tokens
            .items
            .get(self.at)
            .is_some_and(tokens::Token::is_name)
        {
            self.at += 1;
        }
        while matches!(self.word(self.at), ":" | "." | "\\") {
            while matches!(self.word(self.at), ":" | "." | "\\") {
                self.at += 1;
            }
            if self
                .tokens
                .items
                .get(self.at)
                .is_none_or(|token| !token.is_name())
            {
                break;
            }
            self.at += 1;
        }
        self.tokens.items[start..self.at]
            .iter()
            .map(|token| token.text)
            .collect()
    }

    fn skip_modifiers(&mut self) -> Result<(), Failure> {
        while matches!(
            self.word(self.at),
            "export"
                | "default"
                | "public"
                | "private"
                | "protected"
                | "internal"
                | "final"
                | "open"
                | "abstract"
                | "sealed"
                | "static"
                | "pub"
                | "data"
                | "partial"
                | "readonly"
                | "unsafe"
                | "value"
                | "nonisolated"
        ) || self.is_dart_class_modifier()
        {
            let restricted = self.word(self.at) == "pub" && self.word(self.at + 1) == "(";
            self.at += 1;
            if restricted {
                self.at = self
                    .tokens
                    .after_group(self.at)
                    .ok_or_else(|| self.refused(self.at, "invalid visibility"))?;
            }
        }
        Ok(())
    }

    fn is_dart_class_modifier(&self) -> bool {
        if self.input.extension != "dart" {
            return false;
        }
        let next = self.word(self.at + 1);
        match self.word(self.at) {
            "base" | "interface" => matches!(next, "class" | "mixin"),
            "mixin" => next == "class",
            _ => false,
        }
    }

    fn allowed_type(&self, kind: &str) -> bool {
        match self.input.extension {
            "java" | "groovy" => matches!(kind, "class" | "interface" | "record"),
            "kt" => matches!(kind, "class" | "interface" | "object"),
            "cs" => matches!(kind, "class" | "interface" | "record" | "struct"),
            "swift" => matches!(kind, "class" | "struct" | "protocol"),
            "rs" => matches!(kind, "struct" | "enum" | "trait"),
            "php" => matches!(kind, "class" | "interface"),
            "js" | "ts" | "py" | "dart" | "rb" => kind == "class",
            _ => false,
        }
    }

    fn read_name(&mut self) -> Result<String, Failure> {
        let Some(token) = self.tokens.items.get(self.at) else {
            return Err(self.refused(self.at, "a type without a name"));
        };
        if !token.is_name() || Declaration::take_name(token.text) != Some(token.text) {
            return Err(self.refused(self.at, "a type without a name"));
        }
        let name = token.text.to_owned();
        self.at += 1;
        Ok(name)
    }

    fn check_roles(&mut self, markers: &[(String, usize)], name: &str) -> Result<(), Failure> {
        let mut role = None;
        let has_other_role = markers
            .iter()
            .any(|(marker, _)| Role::named(marker).is_some_and(|found| !found.is_http_client()));
        for (marker, at) in markers {
            let Some(next) = Role::named(marker) else {
                continue;
            };
            if has_other_role && next.is_http_client() {
                continue;
            }
            if self.input.source.len() > super::Scan::MAX_SOURCE {
                return Err(self.input.failure(
                    *at,
                    "Role-layout source exceeds 1 MiB",
                    "Split source into bounded role modules.",
                ));
            }
            if role.is_some_and(|previous| previous != next) {
                return Err(self.input.failure(
                    *at,
                    "A type cannot declare conflicting primary roles",
                    "Keep one primary role annotation on each type.",
                ));
            }
            self.input.check_placement(next, name, *at)?;
            self.primary = Some(next);
            role = Some(next);
        }
        Ok(())
    }

    fn read_type_body(&mut self, start: usize) -> Result<(), Failure> {
        if self.input.extension == "py" {
            return self.read_python_body(start);
        }
        if self.input.extension == "rb" {
            return self.read_ruby_body(start, self.at - 2);
        }
        while self.at < self.tokens.items.len() {
            match self.word(self.at) {
                "{" => {
                    let open = self.at;
                    self.at = self
                        .tokens
                        .after_group(self.at)
                        .ok_or_else(|| self.refused(start, "an unclosed class body"))?;
                    self.check_class_body(open + 1, self.at - 1)?;
                    self.skip_type_semicolon();
                    return Ok(());
                }
                "(" | "[" if self.is_javascript() => self.check_javascript_header_word()?,
                "(" | "[" | "<" => {
                    self.at = self
                        .tokens
                        .after_group(self.at)
                        .ok_or_else(|| self.refused(start, "an unclosed type header"))?;
                }
                ";" if matches!(self.input.extension, "rs" | "cs") => {
                    self.at += 1;
                    return Ok(());
                }
                _ => {
                    if self.bodyless_kotlin_ends() {
                        return Ok(());
                    }
                    if self.is_javascript() {
                        self.check_javascript_header_word()?;
                    }
                    if matches!(
                        self.word(self.at),
                        ";" | "=" | "class" | "struct" | "interface" | "fn" | "fun" | "function"
                    ) {
                        return Err(self.refused(self.at, "code outside the class body"));
                    }
                    self.at += 1;
                }
            }
        }
        if self.input.extension == "kt" {
            return Ok(());
        }
        Err(self.refused(start, "a type without a complete body"))
    }

    fn is_javascript(&self) -> bool {
        matches!(self.input.extension, "js" | "ts")
    }

    fn skip_type_semicolon(&mut self) {
        if self.word(self.at) == ";" && matches!(self.input.extension, "js" | "ts" | "cs") {
            self.at += 1;
        }
    }

    fn bodyless_kotlin_ends(&self) -> bool {
        self.input.extension == "kt"
            && self.tokens.new_line_before(self.at)
            && !matches!(self.word(self.at), ":" | "where")
            && !matches!(self.word(self.at - 1), ":" | ",")
            && !self.kotlin_constructor_follows()
    }

    fn kotlin_constructor_follows(&self) -> bool {
        let mut at = self.at;
        while self.tokens.marker_start(at) {
            at = self.tokens.after_marker(at);
        }
        while matches!(
            self.word(at),
            "public" | "private" | "protected" | "internal"
        ) {
            at += 1;
        }
        self.word(at) == "constructor"
    }

    fn read_python_body(&mut self, start: usize) -> Result<(), Failure> {
        while self.at < self.tokens.items.len() && self.word(self.at) != ":" {
            if self.word(self.at) == "(" || self.word(self.at) == "[" {
                let open = self.at;
                self.at = self
                    .tokens
                    .after_group(self.at)
                    .ok_or_else(|| self.refused(start, "an unclosed class header"))?;
                self.check_python_bases(open, self.at)?;
            } else {
                return Err(self.refused(self.at, "an invalid class header"));
            }
        }
        if self.word(self.at) != ":" {
            return Err(self.refused(start, "a class without a body"));
        }
        self.at += 1;
        let body = self.at;
        while self.at < self.tokens.items.len() {
            if self.starts_python_top_level_line(self.at) {
                return self.check_class_body(body, self.at);
            }
            if matches!(self.word(self.at), "(" | "[" | "{") {
                self.at = self
                    .tokens
                    .after_group(self.at)
                    .ok_or_else(|| self.refused(start, "an unclosed class body"))?;
            } else {
                self.at += 1;
            }
        }
        self.check_class_body(body, self.at)
    }

    fn read_implementation(
        &mut self,
        start: usize,
        markers: &[(String, usize)],
    ) -> Result<(), Failure> {
        self.at += 1;
        self.skip_impl_generics(start)?;
        let mut name = self.read_impl_type(start)?;
        if self.word(self.at) == "for" {
            self.at += 1;
            name = self.read_impl_type(start)?;
        }
        if self.word(self.at) != "{" && self.word(self.at) != "where" {
            return Err(self.refused(self.at, "an impl without one concrete target"));
        }
        while self.at < self.tokens.items.len() && self.word(self.at) != "{" {
            if matches!(self.word(self.at), "<" | "(" | "[") {
                self.at = self
                    .tokens
                    .after_group(self.at)
                    .ok_or_else(|| self.refused(start, "an unclosed impl header"))?;
            } else {
                self.at += 1;
            }
        }
        self.check_roles(markers, &name)?;
        self.implementations.push((name, start));
        self.at = self
            .tokens
            .after_group(self.at)
            .ok_or_else(|| self.refused(start, "an unclosed impl body"))?;
        Ok(())
    }

    fn skip_impl_generics(&mut self, start: usize) -> Result<(), Failure> {
        if self.word(self.at) == "<" {
            self.at = self
                .tokens
                .after_group(self.at)
                .ok_or_else(|| self.refused(start, "an unclosed impl generic list"))?;
        }
        Ok(())
    }

    fn read_impl_type(&mut self, start: usize) -> Result<String, Failure> {
        let mut name = self.read_name()?;
        while self.word(self.at) == ":" && self.word(self.at + 1) == ":" {
            self.at += 2;
            name = self.read_name()?;
        }
        self.skip_impl_generics(start)?;
        Ok(name)
    }

    fn finish(self, handler: Option<&str>) -> Result<bool, Failure> {
        let Some(name) = &self.declared else {
            return Err(self.refused(0, "no class declaration"));
        };
        let implementation = if self.input.extension == "go" {
            "a method of"
        } else {
            "an impl for"
        };
        for (target, at) in &self.implementations {
            if target != name {
                return Err(self.refused(*at, &format!("{implementation} another type '{target}'")));
            }
        }
        if let Some(expected) = handler
            && self.primary.is_none()
            && name != expected
        {
            if expected == "Middleware" {
                return Err(self.refused_middleware(name));
            }
            return Err(self.refused(
                0,
                &format!("'{name}' instead of the '{expected}' entry class"),
            ));
        }
        Ok(self.primary.is_some())
    }

    fn refused_middleware(&self, name: &str) -> Failure {
        if self.input.extension == "go" {
            return self.input.failure(
                0,
                format!("Found type '{name}'; Go root middleware declares type Middleware with Before(request) and After(request)"),
                "Rename the type to Middleware. Move the code that runs before the route into its Before method and the code that runs after it into its After method. Tachyon calls both for every request, whatever its HTTP method. A missing Before or After lets the request pass.",
            );
        }
        self.input.failure(
            0,
            format!("Found class '{name}'; root middleware declares class Middleware with before(request) and after(request)"),
            "Rename the class to Middleware. Move the code that runs before the route into before(request) and the code that runs after it into after(request). Tachyon calls both for every request, whatever its HTTP method. A missing before or after lets the request pass.",
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
