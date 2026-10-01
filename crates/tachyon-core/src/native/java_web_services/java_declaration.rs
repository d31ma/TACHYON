use super::java_member::JavaMember;
use super::java_member_kind::JavaMemberKind;
use super::java_value_type::JavaValueType;

pub(super) struct JavaDeclaration {
    annotations: Vec<String>,
    modifiers: Vec<String>,
    rest: String,
}

impl JavaDeclaration {
    const MODIFIERS: [&'static str; 13] = [
        "public",
        "protected",
        "private",
        "static",
        "final",
        "abstract",
        "synchronized",
        "transient",
        "volatile",
        "strictfp",
        "default",
        "sealed",
        "non-sealed",
    ];
    const HIDDEN: [&'static str; 2] = ["private", "static"];
    const NESTED_TYPES: [&'static str; 4] = ["class", "interface", "enum", "record"];
    const LIFECYCLE: [(&'static str, bool); 2] = [("OnStart", false), ("OnStop", true)];
    const MARKERS: [&'static str; 5] =
        ["OnStart", "OnStop", "OnEvent", "LocalState", "SessionState"];
    const KNOWN: [&'static str; 11] = [
        "OnStart",
        "OnStop",
        "OnEvent",
        "Order",
        "InBackground",
        "Needs",
        "LocalState",
        "SessionState",
        "Override",
        "SuppressWarnings",
        "Deprecated",
    ];
    const LISTENER: &'static str = "OnEvent";
    const INSIDE_TYPES: [&'static str; 2] = ["ApplicationEventPublisher", "Behaviors."];
    const READ_ONLY: &'static str = "final";
    const OPENERS: [char; 4] = ['<', '(', '[', '{'];
    const CLOSERS: [char; 4] = ['>', ')', ']', '}'];
    const SUPPORTED: &'static str = "Java web services support @OnStart, @OnStop, @OnEvent, @Order and @InBackground on methods, @LocalState, @SessionState and @Needs on fields, and the behavior annotations such as @Cached, @ClearCache and @AllOrNothing.";
    const VALUES: &'static str = "A Java web service passes int, long, double, float, boolean, their boxed types, String, Object, the models in client/models, and List and Map with String keys of these to the page.";

    pub(super) fn parse(header: &str) -> Self {
        let (annotations, after_annotations) = Self::split_annotations(header);
        let (modifiers, rest) = Self::split_modifiers(after_annotations);
        Self {
            annotations,
            modifiers,
            rest: rest.trim().to_owned(),
        }
    }

    pub(super) fn named_parameters(list: &str) -> Vec<(String, String)> {
        Self::pieces(list)
            .iter()
            .map(|piece| Self::named(piece))
            .filter(|(value_type, _)| !value_type.is_empty())
            .collect()
    }

    pub(super) fn constructor_parameters(&self, class: &str) -> Option<Vec<(String, String)>> {
        let arguments = self
            .rest
            .strip_prefix(class)?
            .trim_start()
            .strip_prefix('(')?;
        let close = arguments.len() - Self::after_closing_parenthesis(arguments).len();
        Some(Self::named_parameters(
            &arguments[..close.saturating_sub(1)],
        ))
    }

    pub(super) fn model_field(&self) -> Option<(String, String)> {
        let declared = self.rest.split('=').next().unwrap_or_default();
        let is_field = [self.is_skipped(), self.is_method()] == [false, false];
        is_field.then(|| Self::type_and_name(declared))
    }

    pub(super) fn refuse_class_annotations(&self) -> Result<(), String> {
        self.refuse_unknown_annotations()
    }

    pub(super) fn member(&self, values: &JavaValueType) -> Result<Option<JavaMember>, String> {
        self.refuse_unknown_annotations()?;
        self.refuse_hidden_markers()?;
        self.visible_member(values)
    }

    fn visible_member(&self, values: &JavaValueType) -> Result<Option<JavaMember>, String> {
        match (self.is_skipped(), self.is_method()) {
            (true, _) => Ok(None),
            (false, true) => self.method(values),
            (false, false) => self.field(values),
        }
    }

    fn is_skipped(&self) -> bool {
        let first = self.rest.split_whitespace().next().unwrap_or_default();
        let stays_inside = Self::stays_inside(first);
        let skipped = [
            !self.is_visible(),
            first.is_empty(),
            Self::NESTED_TYPES.contains(&first),
            self.has_annotation(Self::LISTENER),
            [stays_inside, self.is_method()] == [true, false],
        ];
        skipped.contains(&true)
    }

    fn stays_inside(first: &str) -> bool {
        match first {
            "PreferenceStrings" => true,
            _ => Self::INSIDE_TYPES
                .iter()
                .any(|inside| first.starts_with(inside)),
        }
    }

    fn split_annotations(header: &str) -> (Vec<String>, &str) {
        let mut annotations = Vec::new();
        let mut rest = header.trim_start();
        while let Some(after_at) = rest.strip_prefix('@') {
            let (name, remainder) = Self::annotation(after_at);
            annotations.push(name);
            rest = remainder.trim_start();
        }
        (annotations, rest)
    }

    fn annotation(after_at: &str) -> (String, &str) {
        let name_end = after_at
            .find(Self::ends_annotation_name)
            .unwrap_or(after_at.len());
        let (name, rest) = after_at.split_at(name_end);
        let short_name = name.rsplit('.').next().unwrap_or(name);
        (short_name.to_owned(), Self::without_arguments(rest))
    }

    fn ends_annotation_name(character: char) -> bool {
        !matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.' | '$')
    }

    fn without_arguments(rest: &str) -> &str {
        rest.trim_start()
            .strip_prefix('(')
            .map_or(rest, Self::after_closing_parenthesis)
    }

    fn after_closing_parenthesis(inside: &str) -> &str {
        let close = Self::closing_parenthesis(inside);
        inside.get(close + 1..).unwrap_or_default()
    }

    fn closing_parenthesis(inside: &str) -> usize {
        let mut depth = 1;
        inside
            .char_indices()
            .find(|(_, character)| {
                depth += i32::from(*character == '(') - i32::from(*character == ')');
                depth == 0
            })
            .map_or(inside.len(), |(at, _)| at)
    }

    fn split_modifiers(text: &str) -> (Vec<String>, &str) {
        let mut modifiers = Vec::new();
        let mut rest = text.trim_start();
        while let Some((word, remainder)) = Self::leading_modifier(rest) {
            modifiers.push(word.to_owned());
            rest = remainder.trim_start();
        }
        (modifiers, rest)
    }

    fn leading_modifier(text: &str) -> Option<(&str, &str)> {
        let (word, remainder) = text.split_once(char::is_whitespace)?;
        Self::MODIFIERS.contains(&word).then_some((word, remainder))
    }

    fn is_visible(&self) -> bool {
        !self
            .modifiers
            .iter()
            .any(|modifier| Self::HIDDEN.contains(&modifier.as_str()))
    }

    fn is_method(&self) -> bool {
        let before_value = self.rest.split('=').next().unwrap_or_default();
        before_value.contains('(')
    }

    fn lifecycle(&self) -> Option<bool> {
        Self::LIFECYCLE
            .iter()
            .find(|(name, _)| self.has_annotation(name))
            .map(|(_, destroy)| *destroy)
    }

    fn has_annotation(&self, name: &str) -> bool {
        self.annotations.iter().any(|written| written == name)
    }

    fn marker(&self) -> Option<&String> {
        self.annotations
            .iter()
            .find(|name| Self::MARKERS.contains(&name.as_str()))
    }

    fn refuse_unknown_annotations(&self) -> Result<(), String> {
        let unknown = self
            .annotations
            .iter()
            .find(|name| !Self::KNOWN.contains(&name.as_str()));
        unknown.map_or(Ok(()), |name| {
            Err(format!(
                "@{name} does not work in a Java web service yet. {}",
                Self::SUPPORTED
            ))
        })
    }

    fn refuse_hidden_markers(&self) -> Result<(), String> {
        let hidden_marker = self.marker().filter(|_| !self.is_visible());
        hidden_marker.map_or(Ok(()), |name| {
            Err(format!(
                "A member marked @{name} cannot be private or static, because the page reaches it through the service. Remove private or static."
            ))
        })
    }

    fn method(&self, values: &JavaValueType) -> Result<Option<JavaMember>, String> {
        let open = self.rest.find('(').unwrap_or_default();
        let (value_type, name) = Self::type_and_name(&self.rest[..open]);
        let inside = &self.rest[open + 1..];
        let close = inside.len() - Self::after_closing_parenthesis(inside).len();
        let parameters = Self::parameters(&inside[..close.saturating_sub(1)]);
        let member = JavaMember {
            name,
            value_type,
            parameters,
            kind: JavaMemberKind::Method,
        };
        self.method_member(member, values)
    }

    fn method_member(
        &self,
        member: JavaMember,
        values: &JavaValueType,
    ) -> Result<Option<JavaMember>, String> {
        match (member.value_type.is_empty(), self.lifecycle()) {
            (true, _) => Ok(None),
            (false, Some(destroy)) => Self::hook(member, destroy),
            (false, None) => Self::checked_method(member, values),
        }
    }

    fn hook(member: JavaMember, destroy: bool) -> Result<Option<JavaMember>, String> {
        Self::refuse_hook_parameters(&member)?;
        Ok(Some(JavaMember {
            kind: JavaMemberKind::Hook { destroy },
            ..member
        }))
    }

    fn refuse_hook_parameters(member: &JavaMember) -> Result<(), String> {
        if member.parameters.is_empty() {
            return Ok(());
        }
        Err(format!(
            "The lifecycle method {} takes no arguments. Remove its parameters.",
            member.name
        ))
    }

    fn checked_method(
        member: JavaMember,
        values: &JavaValueType,
    ) -> Result<Option<JavaMember>, String> {
        let returns_value = !JavaValueType::is_nothing(&member.value_type);
        let mut types: Vec<&String> = member.parameters.iter().collect();
        types.extend(returns_value.then_some(&member.value_type));
        Self::require_values(&member.name, &types, values)?;
        Ok(Some(member))
    }

    fn field(&self, values: &JavaValueType) -> Result<Option<JavaMember>, String> {
        let (declared, value) = self
            .rest
            .split_once('=')
            .unwrap_or((self.rest.as_str(), ""));
        Self::refuse_many_names(declared, value)?;
        let (value_type, name) = Self::type_and_name(declared);
        Self::require_values(&name, &[&value_type], values)?;
        let read_only = self
            .modifiers
            .iter()
            .any(|modifier| modifier == Self::READ_ONLY);
        Ok(Some(JavaMember {
            name,
            value_type,
            parameters: Vec::new(),
            kind: JavaMemberKind::Field { read_only },
        }))
    }

    fn refuse_many_names(declared: &str, value: &str) -> Result<(), String> {
        let statement = format!("{declared} {value}");
        if Self::outside_brackets(&statement).contains(',') {
            return Err(String::from(
                "Declare one field per statement in a Java web service, so each field reaches the page by its own name.",
            ));
        }
        Ok(())
    }

    fn outside_brackets(text: &str) -> String {
        let mut depth = 0;
        let mut outside = String::new();
        for character in text.chars() {
            depth += i32::from(Self::OPENERS.contains(&character))
                - i32::from(Self::CLOSERS.contains(&character));
            Self::push_outside(&mut outside, character, depth);
        }
        outside
    }

    fn push_outside(outside: &mut String, character: char, depth: i32) {
        if depth == 0 {
            outside.push(character);
            return;
        }
        outside.push_str(&" ".repeat(character.len_utf8()));
    }

    fn type_and_name(declared: &str) -> (String, String) {
        let declared = declared.trim();
        let split = declared.rfind(char::is_whitespace).unwrap_or_default();
        let (value_type, name) = declared.split_at(split);
        (value_type.trim().to_owned(), name.trim().to_owned())
    }

    fn parameters(list: &str) -> Vec<String> {
        Self::pieces(list)
            .iter()
            .map(|piece| Self::parameter_type(piece))
            .filter(|parameter| !parameter.is_empty())
            .collect()
    }

    fn pieces(list: &str) -> Vec<&str> {
        let outside = Self::outside_brackets(list);
        let mut pieces = Vec::new();
        let mut start = 0;
        for (at, _) in outside.match_indices(',').chain([(list.len(), "")]) {
            pieces.push(&list[start..at]);
            start = at + 1;
        }
        pieces
    }

    fn parameter_type(parameter: &str) -> String {
        Self::named(parameter).0
    }

    fn named(parameter: &str) -> (String, String) {
        let (_, without_annotations) = Self::split_annotations(parameter);
        let without_final = without_annotations
            .trim_start()
            .strip_prefix("final ")
            .unwrap_or(without_annotations);
        Self::type_and_name(without_final)
    }

    fn require_values(name: &str, types: &[&String], values: &JavaValueType) -> Result<(), String> {
        let unsupported = types
            .iter()
            .find(|kind| values.reading(kind, "value").is_none());
        unsupported.map_or(Ok(()), |kind| {
            Err(format!(
                "{name} uses {kind}. {} Make the member private to keep it inside the service.",
                Self::VALUES
            ))
        })
    }
}
