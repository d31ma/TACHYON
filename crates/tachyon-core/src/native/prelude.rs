use std::collections::HashMap as TacMap;

type TacStore = core::cell::RefCell<TacMap<String, String>>;

fn tac_session_values() -> &'static std::thread::LocalKey<TacStore> {
    thread_local! {
        static VALUES: TacStore = core::cell::RefCell::new(TacMap::new());
    }
    &VALUES
}

fn tac_local_values() -> &'static std::thread::LocalKey<TacStore> {
    thread_local! {
        static VALUES: TacStore = core::cell::RefCell::new(tac_store_load());
    }
    &VALUES
}

fn tac_store_base() -> std::path::PathBuf {
    if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(base);
    }
    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        return std::path::PathBuf::from(base);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(home).join(".config")
}

fn tac_store_path() -> std::path::PathBuf {
    tac_store_base()
        .join("tachyon")
        .join(TAC_APPLICATION_ID)
        .join("store.txt")
}

fn tac_store_load() -> TacMap<String, String> {
    let mut values = TacMap::new();
    let Ok(text) = std::fs::read_to_string(tac_store_path()) else {
        return values;
    };
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('\t') {
            values.insert(String::from(key), String::from(value));
        }
    }
    values
}

fn tac_store_save(values: &TacMap<String, String>) {
    let path = tac_store_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut text = String::new();
    for (index, (key, value)) in values.iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        text.push_str(&format!("{key}\t{value}"));
    }
    let _ = std::fs::write(path, text);
}

fn tac_read_store(store: &TacStore, key: &str, fallback: &str) -> String {
    match store.borrow().get(key) {
        Some(value) => value.clone(),
        None => String::from(fallback),
    }
}

pub fn tac_local(key: &str, fallback: &str) -> String {
    tac_local_values().with(|values| tac_read_store(values, key, fallback))
}

pub fn tac_set_local(key: &str, value: &str) {
    tac_local_values().with(|values| {
        values
            .borrow_mut()
            .insert(String::from(key), String::from(value));
        tac_store_save(&values.borrow());
    });
}

pub fn tac_session(key: &str, fallback: &str) -> String {
    tac_session_values().with(|values| tac_read_store(values, key, fallback))
}

pub fn tac_set_session(key: &str, value: &str) {
    tac_session_values().with(|values| {
        values
            .borrow_mut()
            .insert(String::from(key), String::from(value));
    });
}

#[derive(Clone, Default)]
pub enum TacValue {
    #[default]
    Null,
    Flag(bool),
    Int(i64),
    Float(f64),
    Text(String),
    List(Vec<TacValue>),
    Object(Vec<(String, TacValue)>),
}

impl TacValue {
    pub fn as_int(&self) -> i64 {
        match self {
            Self::Int(value) => *value,
            Self::Float(value) => *value as i64,
            Self::Flag(value) => i64::from(*value),
            Self::Text(value) => value.parse().unwrap_or_default(),
            _ => 0,
        }
    }

    pub fn as_float(&self) -> f64 {
        match self {
            Self::Int(value) => *value as f64,
            Self::Float(value) => *value,
            Self::Text(value) => value.parse().unwrap_or_default(),
            _ => 0.0,
        }
    }

    pub fn as_flag(&self) -> bool {
        match self {
            Self::Flag(value) => *value,
            Self::Int(value) => *value != 0,
            Self::Float(value) => *value != 0.0,
            Self::Text(value) => !value.is_empty(),
            Self::List(values) => !values.is_empty(),
            Self::Object(values) => !values.is_empty(),
            Self::Null => false,
        }
    }

    pub fn as_text(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::Int(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::Flag(value) => value.to_string(),
            _ => String::new(),
        }
    }
}

pub struct TacListener {
    pub events: &'static [&'static str],
    pub replay: bool,
    pub order: i64,
    pub condition: Option<&'static str>,
    pub models: Option<&'static str>,
    pub returns: Option<&'static str>,
    pub asynchronous: bool,
    pub invoke: Box<dyn Fn(&TacValue) -> Result<TacValue, &'static str>>,
}

pub trait TacModel: Sized {
    fn read(value: &TacValue) -> Self;
    fn write(self) -> TacValue;
}

impl TacModel for TacValue {
    fn read(value: &TacValue) -> Self {
        value.clone()
    }
    fn write(self) -> TacValue {
        self
    }
}

impl TacModel for String {
    fn read(value: &TacValue) -> Self {
        value.as_text()
    }
    fn write(self) -> TacValue {
        TacValue::Text(self)
    }
}

impl TacModel for i64 {
    fn read(value: &TacValue) -> Self {
        value.as_int()
    }
    fn write(self) -> TacValue {
        TacValue::Int(self)
    }
}

impl TacModel for i32 {
    fn read(value: &TacValue) -> Self {
        value.as_int() as i32
    }
    fn write(self) -> TacValue {
        TacValue::Int(i64::from(self))
    }
}

impl TacModel for f64 {
    fn read(value: &TacValue) -> Self {
        value.as_float()
    }
    fn write(self) -> TacValue {
        TacValue::Float(self)
    }
}

impl TacModel for bool {
    fn read(value: &TacValue) -> Self {
        value.as_flag()
    }
    fn write(self) -> TacValue {
        TacValue::Flag(self)
    }
}

impl<T: TacModel> TacModel for Vec<T> {
    fn read(value: &TacValue) -> Self {
        match value {
            TacValue::List(items) => items.iter().map(T::read).collect(),
            _ => Vec::new(),
        }
    }
    fn write(self) -> TacValue {
        TacValue::List(self.into_iter().map(T::write).collect())
    }
}

impl<T: TacModel> TacModel for Option<T> {
    fn read(value: &TacValue) -> Self {
        match value {
            TacValue::Null => None,
            other => Some(T::read(other)),
        }
    }
    fn write(self) -> TacValue {
        match self {
            Some(value) => value.write(),
            None => TacValue::Null,
        }
    }
}

pub fn tac_read_field<T: TacModel>(value: &TacValue, name: &str) -> T {
    let TacValue::Object(entries) = value else {
        return T::read(&TacValue::Null);
    };
    match entries.iter().find(|(key, _)| key == name) {
        Some((_, found)) => T::read(found),
        None => T::read(&TacValue::Null),
    }
}

pub enum TacMember {
    Field {
        read: Box<dyn Fn() -> TacValue>,
        write: Option<Box<dyn Fn(TacValue)>>,
    },
    Method(Box<dyn Fn(Vec<TacValue>) -> TacValue>),
    FallibleMethod(Box<dyn Fn(Vec<TacValue>) -> Result<TacValue, &'static str>>),
    Listener(TacListener),
    Hook(bool, Box<dyn Fn() -> Result<(), &'static str>>),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ApplicationEventPublisher;

impl ApplicationEventPublisher {
    pub fn new() -> Self {
        Self
    }

    pub fn publish_event<T: TacModel>(&self, value: T, retain: bool) {
        let full_name = core::any::type_name::<T>();
        let name = full_name.rsplit("::").next().unwrap_or(full_name);
        let starts_upper = name.starts_with(|first: char| first.is_ascii_uppercase());
        let is_model =
            starts_upper && !full_name.contains('<') && !matches!(name, "String" | "TacValue");
        if !is_model {
            tac_event_failure();
            return;
        }
        tac_event_publish(name, value.write(), retain);
    }
}

struct TacScanner<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> TacScanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            bytes: source.as_bytes(),
            position: 0,
        }
    }

    fn skip_space(&mut self) {
        while self.position < self.bytes.len() && self.bytes[self.position].is_ascii_whitespace() {
            self.position += 1;
        }
    }

    fn peek(&mut self) -> u8 {
        self.skip_space();
        *self.bytes.get(self.position).unwrap_or(&0)
    }

    fn value(&mut self) -> TacValue {
        match self.peek() {
            b'{' => self.object_value(),
            b'[' => self.list(),
            b'"' => TacValue::Text(self.text()),
            b't' => {
                self.position += 4;
                TacValue::Flag(true)
            }
            b'f' => {
                self.position += 5;
                TacValue::Flag(false)
            }
            b'n' => {
                self.position += 4;
                TacValue::Null
            }
            _ => self.number(),
        }
    }

    fn object(&mut self) -> Vec<(String, TacValue)> {
        let mut entries = Vec::new();
        if self.peek() != b'{' {
            return entries;
        }
        self.position += 1;
        loop {
            if self.peek() == b'}' {
                self.position += 1;
                return entries;
            }
            let key = self.text();
            if self.peek() == b':' {
                self.position += 1;
            }
            entries.push((key, self.value()));
            if self.peek() == b',' {
                self.position += 1;
                continue;
            }
            if self.peek() == b'}' {
                self.position += 1;
            }
            return entries;
        }
    }

    fn object_value(&mut self) -> TacValue {
        TacValue::Object(self.object())
    }

    fn list(&mut self) -> TacValue {
        let mut values = Vec::new();
        self.position += 1;
        loop {
            if self.peek() == b']' {
                self.position += 1;
                return TacValue::List(values);
            }
            values.push(self.value());
            if self.peek() == b',' {
                self.position += 1;
                continue;
            }
            if self.peek() == b']' {
                self.position += 1;
            }
            return TacValue::List(values);
        }
    }

    fn text(&mut self) -> String {
        let mut text = String::new();
        if self.peek() != b'"' {
            return text;
        }
        self.position += 1;
        while self.position < self.bytes.len() {
            let byte = self.bytes[self.position];
            self.position += 1;
            match byte {
                b'"' => return text,
                b'\\' => text.push(self.escaped_character()),
                _ => text.push_str(self.unescaped_character(byte)),
            }
        }
        text
    }

    fn escaped_character(&mut self) -> char {
        let escape = *self.bytes.get(self.position).unwrap_or(&b'"');
        self.position += 1;
        match escape {
            b'n' => '\n',
            b't' => '\t',
            b'r' => '\r',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'u' => self.unicode_escape(),
            other => other as char,
        }
    }

    fn unicode_escape(&mut self) -> char {
        let digits = self.bytes.get(self.position..self.position + 4);
        self.position += 4;
        let Some(digits) = digits else {
            return '\u{fffd}';
        };
        let Ok(digits) = core::str::from_utf8(digits) else {
            return '\u{fffd}';
        };
        let code = u32::from_str_radix(digits, 16).unwrap_or(0xFFFD);
        char::from_u32(code).unwrap_or('\u{fffd}')
    }

    fn unescaped_character(&mut self, first_byte: u8) -> &'a str {
        let start = self.position - 1;
        let width = match first_byte {
            0x00..=0x7F => 1,
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            _ => 4,
        };
        self.position = start + width;
        let bytes = self.bytes.get(start..self.position).unwrap_or(&[]);
        core::str::from_utf8(bytes).unwrap_or("\u{fffd}")
    }

    fn number(&mut self) -> TacValue {
        self.skip_space();
        let start = self.position;
        while self.position < self.bytes.len()
            && matches!(
                self.bytes[self.position],
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
            )
        {
            self.position += 1;
        }
        let bytes = self.bytes.get(start..self.position).unwrap_or(&[]);
        let raw = core::str::from_utf8(bytes).unwrap_or("0");
        match raw.parse::<i64>() {
            Ok(integer) => TacValue::Int(integer),
            Err(_) => TacValue::Float(raw.parse().unwrap_or(0.0)),
        }
    }
}

fn tac_quote(value: &str) -> String {
    let mut quoted = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            '\r' => quoted.push_str("\\r"),
            other if (other as u32) < 0x20 => {
                quoted.push_str(&format!("\\u{:04x}", other as u32));
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

fn tac_write_float(value: f64) -> String {
    if !value.is_finite() {
        return String::from("null");
    }
    if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
        return (value as i64).to_string();
    }
    value.to_string()
}

fn tac_write(value: &TacValue) -> String {
    match value {
        TacValue::Null => String::from("null"),
        TacValue::Flag(inner) => inner.to_string(),
        TacValue::Int(inner) => inner.to_string(),
        TacValue::Float(inner) => tac_write_float(*inner),
        TacValue::Text(inner) => tac_quote(inner),
        TacValue::List(values) => {
            let mut items = Vec::with_capacity(values.len());
            for item in values {
                items.push(tac_write(item));
            }
            format!("[{}]", items.join(","))
        }
        TacValue::Object(values) => {
            let mut items = Vec::with_capacity(values.len());
            for (key, item) in values {
                items.push(format!("{}:{}", tac_quote(key), tac_write(item)));
            }
            format!("{{{}}}", items.join(","))
        }
    }
}

fn tac_bounded_json(raw: &str) -> bool {
    let mut depth = 0u8;
    let mut quoted = false;
    let mut escaped = false;
    for byte in raw.bytes() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > 64 {
                    return false;
                }
            }
            b'}' | b']' => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    depth == 0 && !quoted && raw.trim_start().starts_with('{')
}

fn tac_entry<'a>(entries: &'a [(String, TacValue)], name: &str) -> Option<&'a TacValue> {
    entries
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn tac_entry_text<'a>(entries: &'a [(String, TacValue)], name: &str) -> Option<&'a String> {
    match tac_entry(entries, name) {
        Some(TacValue::Text(text)) => Some(text),
        _ => None,
    }
}

fn tac_arguments(request: &[(String, TacValue)]) -> Vec<TacValue> {
    match tac_entry(request, "args") {
        Some(TacValue::List(values)) => values.clone(),
        _ => Vec::new(),
    }
}

fn tac_active_owner(owner: &str) -> Option<std::rc::Rc<TacActivation>> {
    tac_active_owners().with(|slot| slot.borrow().get(owner).cloned())
}

fn tac_set_executing(activation: &std::rc::Rc<TacActivation>) {
    tac_executing().with(|slot| *slot.borrow_mut() = Some(activation.clone()));
}

fn tac_assign_properties(
    activation: &TacActivation,
    service: &str,
    properties: &[(String, TacValue)],
) {
    let required = tac_service_required(service);
    for (key, value) in properties {
        if !required.contains(&key.as_str()) && !required.contains(&"*") {
            continue;
        }
        let member = activation
            .members
            .iter()
            .find(|(name, _)| *name == key.as_str());
        if let Some((
            _,
            TacMember::Field {
                write: Some(write), ..
            },
        )) = member
        {
            write(value.clone());
        }
    }
}

fn tac_run_initializers(activation: &TacActivation) -> bool {
    for (_, member) in &activation.members {
        if let TacMember::Hook(false, invoke) = member {
            if invoke().is_err() {
                return false;
            }
        }
    }
    true
}

fn tac_activate(
    request: &[(String, TacValue)],
    route: &String,
    service: &String,
    owner: &String,
) -> Result<(), String> {
    let existing = tac_active_owner(owner);
    let is_current = existing
        .as_ref()
        .is_some_and(|active| active.route == *route && active.service == *service);
    if is_current {
        return Ok(());
    }
    if existing.is_some() {
        tac_dispose_owner(owner);
    }
    if tac_active_owners().with(|slot| slot.borrow().len()) >= 256 {
        return Err(String::from("{\"error\":\"Too many native activations.\"}"));
    }
    let (members, capabilities) = NativeCapabilities::create(owner, || {
        NativePageServices::build(route, owner, || tac_service_members(service))
    });
    let members = members.map_err(|_| {
        capabilities.dispose();
        String::from("{\"error\":\"Native service construction failed.\"}")
    })?;
    let Some(members) = members else {
        NativePageServices::rollback(owner);
        capabilities.dispose();
        return Err(String::from(
            "{\"error\":\"Unknown native service.\",\"code\":\"TY_NATIVE_SERVICE\"}",
        ));
    };
    let activation = std::rc::Rc::new(TacActivation::new(
        route,
        service,
        owner,
        members,
        capabilities,
    ));
    tac_active_owners().with(|slot| slot.borrow_mut().insert(owner.clone(), activation.clone()));
    tac_set_executing(&activation);
    if let Some(TacValue::Object(properties)) = tac_entry(request, "props") {
        tac_assign_properties(&activation, service, properties);
    }
    if !tac_run_initializers(&activation) {
        NativePageServices::rollback(owner);
        tac_dispose_owner(owner);
        return Err(String::from("{\"error\":\"Native initializer failed.\"}"));
    }
    NativePageServices::commit(owner);
    tac_drain(&activation);
    Ok(())
}

fn tac_describe_members(members: &[(&'static str, TacMember)]) -> String {
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let mut listeners = Vec::new();
    for (name, member) in members {
        match member {
            TacMember::Field { .. } => fields.push(TacValue::Text(String::from(*name))),
            TacMember::Method(_) | TacMember::FallibleMethod(_) => {
                methods.push(TacValue::Text(String::from(*name)))
            }
            TacMember::Listener(listener) => listeners.push(tac_describe_listener(name, listener)),
            TacMember::Hook(..) => {}
        }
    }
    format!(
        "{{\"value\":{{\"fields\":{},\"methods\":{},\"listeners\":{}}}}}",
        tac_write(&TacValue::List(fields)),
        tac_write(&TacValue::List(methods)),
        tac_write(&TacValue::List(listeners))
    )
}

fn tac_describe_listener(key: &str, listener: &TacListener) -> TacValue {
    let events = listener
        .events
        .iter()
        .map(|event| TacValue::Text(String::from(*event)))
        .collect();
    let condition = listener.condition.map_or(TacValue::Null, |condition| {
        TacScanner::new(condition).value()
    });
    let models = listener
        .models
        .map_or(TacValue::Null, |models| TacScanner::new(models).value());
    TacValue::Object(vec![
        (String::from("models"), models),
        (String::from("key"), TacValue::Text(key.to_owned())),
        (String::from("events"), TacValue::List(events)),
        (String::from("replay"), TacValue::Flag(listener.replay)),
        (String::from("order"), TacValue::Int(listener.order)),
        (
            String::from("asynchronous"),
            TacValue::Flag(listener.asynchronous),
        ),
        (String::from("condition"), condition),
    ])
}

fn tac_receive(owner: &TacActivation, key: &str, request: &[(String, TacValue)]) -> String {
    let found = owner
        .members
        .iter()
        .find_map(|(name, member)| match member {
            TacMember::Listener(listener) if *name == key => Some(listener),
            _ => None,
        });
    let Some(listener) = found else {
        return tac_error(&format!("Unknown companion listener: {key}"));
    };
    let value = tac_entry(request, "value")
        .cloned()
        .unwrap_or(TacValue::Null);
    let Ok(result) = (listener.invoke)(&value) else {
        return tac_error("Native event listener failed.");
    };
    if !listener.asynchronous {
        tac_republish(&result, listener.returns);
    }
    String::from("{\"value\":null}")
}

fn tac_error(message: &str) -> String {
    format!("{{\"error\":{}}}", tac_quote(message))
}

fn tac_use_member(
    operation: &str,
    name: &str,
    member: &TacMember,
    request: &[(String, TacValue)],
) -> String {
    match (operation, member) {
        ("get", TacMember::Field { read, .. }) => format!("{{\"value\":{}}}", tac_write(&read())),
        (
            "set",
            TacMember::Field {
                write: Some(write), ..
            },
        ) => {
            write(
                tac_entry(request, "value")
                    .cloned()
                    .unwrap_or(TacValue::Null),
            );
            String::from("{\"value\":null}")
        }
        ("set", TacMember::Field { write: None, .. }) => {
            tac_error(&format!("Companion field is read-only: {name}"))
        }
        ("call", TacMember::Method(invoke)) => {
            format!(
                "{{\"value\":{}}}",
                tac_write(&invoke(tac_arguments(request)))
            )
        }
        ("call", TacMember::FallibleMethod(invoke)) => match invoke(tac_arguments(request)) {
            Ok(value) => format!("{{\"value\":{}}}", tac_write(&value)),
            Err(_) => String::from("{\"error\":\"Native companion operation failed.\"}"),
        },
        _ => tac_error(&format!(
            "Companion member does not support {operation}: {name}"
        )),
    }
}

fn tac_too_large(subject: &str, size: usize, limit: usize) -> String {
    tac_error(&format!(
        "The {subject} is {size} bytes. The limit is {limit} bytes. Pass less data to the service."
    ))
}

fn tac_handle(raw: &str) -> String {
    const START_UP_REQUEST_LIMIT: usize = 1_048_576;
    const MEMBER_REQUEST_LIMIT: usize = 65_536;
    if raw.len() > START_UP_REQUEST_LIMIT {
        return tac_too_large("request", raw.len(), START_UP_REQUEST_LIMIT);
    }
    if !tac_bounded_json(raw) {
        return String::from("{\"error\":\"Invalid companion request.\"}");
    }
    let request = TacScanner::new(raw).object();
    let Some(operation) = tac_entry_text(&request, "op") else {
        return String::from("{\"error\":\"Companion request has no operation.\"}");
    };
    if operation != "init" && raw.len() > MEMBER_REQUEST_LIMIT {
        let subject = tac_entry_text(&request, "name").unwrap_or(operation);
        return tac_too_large(
            &format!("request to {subject}"),
            raw.len(),
            MEMBER_REQUEST_LIMIT,
        );
    }
    if operation == "host-dispose" {
        tac_dispose_all();
        return String::from("{\"value\":null}");
    }
    let Some(route) = tac_entry_text(&request, "route") else {
        return String::from(
            "{\"error\":\"Missing companion route.\",\"code\":\"TY_NATIVE_ROUTE\"}",
        );
    };
    let Some(service) = tac_entry_text(&request, "service") else {
        return String::from(
            "{\"error\":\"Missing companion name.\",\"code\":\"TY_NATIVE_SERVICE\"}",
        );
    };
    let Some(owner) = tac_entry_text(&request, "owner").filter(|owner| tac_owner_id(owner)) else {
        return String::from(
            "{\"error\":\"Missing native owner identity.\",\"code\":\"TY_NATIVE_OWNER\"}",
        );
    };
    tac_handle_owned(&request, operation, route, service, owner)
}

fn tac_handle_owned(
    request: &[(String, TacValue)],
    operation: &str,
    route: &String,
    service: &String,
    owner: &String,
) -> String {
    if operation == "init" {
        if let Err(answer) = tac_activate(request, route, service, owner) {
            return answer;
        }
    }
    let activation = tac_active_owner(owner)
        .filter(|active| active.route == *route && active.service == *service);
    let Some(activation) = activation else {
        return String::from("{\"error\":\"Stale native owner.\",\"code\":\"TY_NATIVE_OWNER\"}");
    };
    if operation == "dispose" {
        tac_dispose_owner(owner);
        return String::from("{\"value\":null}");
    }
    tac_set_executing(&activation);
    if operation == "init" {
        return tac_describe_members(&activation.members);
    }
    let Some(name) = tac_entry_text(request, "name") else {
        return String::from("{\"error\":\"Companion request has no member name.\"}");
    };
    if operation == "event" {
        return tac_receive(&activation, name, request);
    }
    let Some((_, member)) = activation.members.iter().find(|(key, _)| key == name) else {
        return tac_error(&format!("Unknown companion member: {name}"));
    };
    tac_use_member(operation, name, member, request)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tac_native_invoke(
    request: *const core::ffi::c_char,
) -> *mut core::ffi::c_char {
    let text = if request.is_null() {
        String::from("{}")
    } else {
        let bytes = unsafe { core::ffi::CStr::from_ptr(request) }.to_bytes();
        String::from_utf8_lossy(bytes).into_owned()
    };
    let answer = tac_invoke_owned(&text);
    match std::ffi::CString::new(answer) {
        Ok(owned) => owned.into_raw(),
        Err(_) => std::ffi::CString::new("{\"error\":\"Companion answer contains a NUL byte.\"}")
            .unwrap_or_default()
            .into_raw(),
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "tachyon")]
unsafe extern "C" {
    fn tac_host_emit(payload: *const u8, length: usize);
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn tac_wasm_alloc(length: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(length.max(1));
    let pointer = buffer.as_mut_ptr();
    core::mem::forget(buffer);
    pointer
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tac_wasm_dealloc(pointer: *mut u8, length: usize) {
    drop(unsafe { Vec::from_raw_parts(pointer, 0, length.max(1)) });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tac_native_free(answer: *mut core::ffi::c_char) {
    if !answer.is_null() {
        drop(unsafe { std::ffi::CString::from_raw(answer) });
    }
}

type TacEmit = extern "C" fn(*const core::ffi::c_char);

fn tac_emit_slot() -> &'static std::sync::Mutex<Option<TacEmit>> {
    static SLOT: std::sync::Mutex<Option<TacEmit>> = std::sync::Mutex::new(None);
    &SLOT
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tac_native_set_emit(emit: Option<TacEmit>) {
    if let Ok(mut installed) = tac_emit_slot().lock() {
        *installed = emit;
    }
}

fn tac_event_payload(
    route: &str,
    service: &str,
    owner: &str,
    name: &str,
    value: &TacValue,
    retain: bool,
) -> String {
    format!(
        "{{\"route\":{},\"service\":{},\"owner\":{},\"name\":{},\"value\":{},\"retain\":{retain}}}",
        tac_quote(route),
        tac_quote(service),
        tac_quote(owner),
        tac_quote(name),
        tac_write(value)
    )
}

fn tac_send_event(
    route: &str,
    service: &str,
    owner: &str,
    name: &str,
    value: &TacValue,
    retain: bool,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let payload = tac_event_payload(route, service, owner, name, value, retain);
        unsafe { tac_host_emit(payload.as_ptr(), payload.len()) };
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let installed = tac_emit_slot().lock().ok().and_then(|installed| *installed);
        let Some(emit) = installed else {
            return;
        };
        let payload = tac_event_payload(route, service, owner, name, value, retain);
        if let Ok(owned) = std::ffi::CString::new(payload) {
            emit(owned.as_ptr());
        }
    }
}

struct TacActivation {
    route: String,
    service: String,
    owner: String,
    members: Vec<(&'static str, TacMember)>,
    capabilities: std::rc::Rc<NativeCapabilityScope>,
    listeners: Vec<usize>,
    disposed: core::cell::Cell<bool>,
    queue: core::cell::RefCell<std::collections::VecDeque<(String, TacValue, bool)>>,
    retained: core::cell::RefCell<TacMap<String, TacValue>>,
    deferred: core::cell::RefCell<Vec<(usize, TacValue)>>,
}

impl TacActivation {
    fn new(
        route: &str,
        service: &str,
        owner: &str,
        members: Vec<(&'static str, TacMember)>,
        capabilities: std::rc::Rc<NativeCapabilityScope>,
    ) -> Self {
        let mut listeners = Vec::new();
        for (index, (_, member)) in members.iter().enumerate() {
            if matches!(member, TacMember::Listener(_)) {
                listeners.push(index);
            }
        }
        listeners.sort_by_key(|index| match &members[*index].1 {
            TacMember::Listener(listener) => listener.order,
            _ => 0,
        });
        Self {
            route: route.to_owned(),
            service: service.to_owned(),
            owner: owner.to_owned(),
            members,
            capabilities,
            listeners,
            disposed: core::cell::Cell::new(false),
            queue: core::cell::RefCell::new(std::collections::VecDeque::new()),
            retained: core::cell::RefCell::new(TacMap::new()),
            deferred: core::cell::RefCell::new(Vec::new()),
        }
    }
}

type TacOwners = core::cell::RefCell<TacMap<String, std::rc::Rc<TacActivation>>>;
type TacExecuting = core::cell::RefCell<Option<std::rc::Rc<TacActivation>>>;

fn tac_active_owners() -> &'static std::thread::LocalKey<TacOwners> {
    thread_local! {
        static OWNERS: TacOwners = core::cell::RefCell::new(TacMap::new());
    }
    &OWNERS
}

fn tac_executing() -> &'static std::thread::LocalKey<TacExecuting> {
    thread_local! {
        static EXECUTING: TacExecuting = const { core::cell::RefCell::new(None) };
    }
    &EXECUTING
}

fn tac_handling() -> &'static std::thread::LocalKey<core::cell::Cell<bool>> {
    thread_local! {
        static HANDLING: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    }
    &HANDLING
}

fn tac_owner_id(text: &str) -> bool {
    if text.len() != 36 {
        return false;
    }
    for (index, byte) in text.bytes().enumerate() {
        let is_hyphen_position = matches!(index, 8 | 13 | 18 | 23);
        let is_valid = if is_hyphen_position {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        };
        if !is_valid {
            return false;
        }
    }
    true
}

fn tac_event_failure() {
    eprintln!("Tachyon native event delivery failed.");
}

fn tac_publishable(value: &TacValue, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    match value {
        TacValue::Float(value) => value.is_finite(),
        TacValue::List(values) => values.iter().all(|value| tac_publishable(value, depth + 1)),
        TacValue::Object(values) => values
            .iter()
            .all(|(_, value)| tac_publishable(value, depth + 1)),
        _ => true,
    }
}

fn tac_event_publish(name: &str, value: TacValue, retain: bool) {
    let executing = tac_executing().with(|slot| slot.borrow().clone());
    let Some(owner) = executing.filter(|owner| !owner.disposed.get()) else {
        tac_event_failure();
        return;
    };
    if !tac_publishable(&value, 0)
        || tac_write(&value).len() > 65_536
        || owner.queue.borrow().len() >= 128
    {
        tac_event_failure();
        return;
    }
    owner
        .queue
        .borrow_mut()
        .push_back((name.to_owned(), value, retain));
}

enum TacOperand<'a> {
    Absent,
    Borrowed(&'a TacValue),
    Owned(TacValue),
}

impl TacOperand<'_> {
    fn get(&self) -> Option<&TacValue> {
        match self {
            Self::Absent => None,
            Self::Borrowed(value) => Some(value),
            Self::Owned(value) => Some(value),
        }
    }
}

fn tac_member<'a>(value: &'a TacValue, key: &str) -> Option<&'a TacValue> {
    match value {
        TacValue::Object(entries) => tac_entry(entries, key),
        TacValue::List(values) => match key.parse::<usize>() {
            Ok(index) => values.get(index),
            Err(_) => None,
        },
        _ => None,
    }
}

fn tac_segment(value: &TacValue) -> Option<String> {
    match value {
        TacValue::Text(text) => Some(text.clone()),
        TacValue::Int(number) => Some(number.to_string()),
        TacValue::Float(number) if number.fract() == 0.0 => Some((*number as i64).to_string()),
        _ => None,
    }
}

fn tac_node_text(node: &TacValue, key: &str) -> String {
    tac_member(node, key)
        .map(TacValue::as_text)
        .unwrap_or_default()
}

fn tac_operand<'a>(node: &TacValue, value: &'a TacValue, event: &str) -> TacOperand<'a> {
    let kind = tac_node_text(node, "k");
    if kind == "lit" {
        return TacOperand::Owned(tac_member(node, "v").cloned().unwrap_or(TacValue::Null));
    }
    if kind == "event" {
        return TacOperand::Owned(TacValue::Text(event.to_owned()));
    }
    let mut current = value;
    let Some(TacValue::List(path)) = tac_member(node, "path") else {
        return TacOperand::Borrowed(current);
    };
    for segment in path {
        let Some(key) = tac_segment(segment) else {
            return TacOperand::Absent;
        };
        let Some(next) = tac_member(current, &key) else {
            return TacOperand::Absent;
        };
        current = next;
    }
    TacOperand::Borrowed(current)
}

fn tac_number(value: Option<&TacValue>) -> Option<f64> {
    match value {
        Some(TacValue::Int(number)) => Some(*number as f64),
        Some(TacValue::Float(number)) => Some(*number),
        _ => None,
    }
}

fn tac_truthy(value: Option<&TacValue>) -> bool {
    match value {
        None | Some(TacValue::Null) => false,
        Some(TacValue::Flag(flag)) => *flag,
        Some(TacValue::Text(text)) => !text.is_empty(),
        Some(TacValue::Int(number)) => *number != 0,
        Some(TacValue::Float(number)) => *number != 0.0 && !number.is_nan(),
        Some(_) => true,
    }
}

fn tac_identical_lists(left: &[TacValue], right: &[TacValue]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    for (left, right) in left.iter().zip(right.iter()) {
        if !tac_identical(Some(left), Some(right)) {
            return false;
        }
    }
    true
}

fn tac_identical_objects(left: &[(String, TacValue)], right: &[(String, TacValue)]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    for (key, value) in left {
        let Some(held) = tac_entry(right, key) else {
            return false;
        };
        if !tac_identical(Some(value), Some(held)) {
            return false;
        }
    }
    true
}

fn tac_identical(left: Option<&TacValue>, right: Option<&TacValue>) -> bool {
    match (left, right) {
        (None, None) => true,
        (None, _) | (_, None) => false,
        (Some(TacValue::Null), Some(TacValue::Null)) => true,
        (Some(TacValue::Flag(left)), Some(TacValue::Flag(right))) => left == right,
        (Some(TacValue::Text(left)), Some(TacValue::Text(right))) => left == right,
        (Some(TacValue::List(left)), Some(TacValue::List(right))) => {
            tac_identical_lists(left, right)
        }
        (Some(TacValue::Object(left)), Some(TacValue::Object(right))) => {
            tac_identical_objects(left, right)
        }
        (left, right) => match (tac_number(left), tac_number(right)) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        },
    }
}

fn tac_order(left: Option<&TacValue>, right: Option<&TacValue>) -> Option<core::cmp::Ordering> {
    if let (Some(left), Some(right)) = (tac_number(left), tac_number(right)) {
        return left.partial_cmp(&right);
    }
    match (left, right) {
        (Some(TacValue::Text(left)), Some(TacValue::Text(right))) => Some(left.cmp(right)),
        _ => None,
    }
}

fn tac_orders(operation: &str, left: Option<&TacValue>, right: Option<&TacValue>) -> bool {
    let Some(order) = tac_order(left, right) else {
        return false;
    };
    match operation {
        "lt" => order.is_lt(),
        "le" => order.is_le(),
        "gt" => order.is_gt(),
        _ => order.is_ge(),
    }
}

fn tac_side_matches(matcher: &TacValue, side: &str, value: &TacValue, event: &str) -> bool {
    tac_member(matcher, side).is_some_and(|node| tac_matches(node, value, event))
}

fn tac_compare(matcher: &TacValue, value: &TacValue, event: &str) -> bool {
    let (Some(left), Some(right)) = (tac_member(matcher, "l"), tac_member(matcher, "r")) else {
        return false;
    };
    let left = tac_operand(left, value, event);
    let right = tac_operand(right, value, event);
    let operation = tac_node_text(matcher, "op");
    match operation.as_str() {
        "eq" => tac_identical(left.get(), right.get()),
        "ne" => !tac_identical(left.get(), right.get()),
        other => tac_orders(other, left.get(), right.get()),
    }
}

fn tac_matches(matcher: &TacValue, value: &TacValue, event: &str) -> bool {
    let kind = tac_node_text(matcher, "k");
    match kind.as_str() {
        "and" => {
            tac_side_matches(matcher, "l", value, event)
                && tac_side_matches(matcher, "r", value, event)
        }
        "or" => {
            tac_side_matches(matcher, "l", value, event)
                || tac_side_matches(matcher, "r", value, event)
        }
        "not" => !tac_side_matches(matcher, "e", value, event),
        "truthy" => tac_member(matcher, "e")
            .is_some_and(|node| tac_truthy(tac_operand(node, value, event).get())),
        "cmp" => tac_compare(matcher, value, event),
        _ => false,
    }
}

fn tac_republish(result: &TacValue, returns: Option<&str>) {
    let Some(name) = returns else {
        return;
    };
    let items = match result {
        TacValue::List(values) => values.clone(),
        other => vec![other.clone()],
    };
    for item in items.into_iter().take(32) {
        if !matches!(item, TacValue::Null) {
            tac_event_publish(name, item, false);
        }
    }
}

fn tac_fits(models: &TacValue, shape: &str, value: &TacValue) -> bool {
    if let Some(inner) = shape.strip_prefix('?') {
        return matches!(value, TacValue::Null) || tac_fits(models, inner, value);
    }
    if let Some(item) = shape.strip_prefix("[]") {
        let TacValue::List(items) = value else {
            return false;
        };
        return items.iter().all(|entry| tac_fits(models, item, entry));
    }
    match shape {
        "any" => return true,
        "string" => return matches!(value, TacValue::Text(_)),
        "integer" => return matches!(value, TacValue::Int(_)),
        "number" => return matches!(value, TacValue::Int(_) | TacValue::Float(_)),
        "boolean" => return matches!(value, TacValue::Flag(_)),
        _ => {}
    }
    let (TacValue::Object(declared), TacValue::Object(entries)) = (models, value) else {
        return false;
    };
    let Some(TacValue::Object(model)) = tac_entry(declared, shape) else {
        return false;
    };
    let Some(TacValue::Object(fields)) = tac_entry(model, "fields") else {
        return false;
    };
    let open = matches!(tac_entry(model, "open"), Some(TacValue::Flag(true)));
    if !open
        && entries
            .iter()
            .any(|(key, _)| tac_entry(fields, key).is_none())
    {
        return false;
    }
    for (name, declared_shape) in fields {
        let TacValue::Text(field) = declared_shape else {
            return false;
        };
        let required = field.strip_prefix('!');
        let Some(found) = tac_entry(entries, name) else {
            if required.is_some() {
                return false;
            }
            continue;
        };
        if !tac_fits(models, required.unwrap_or(field), found) {
            return false;
        }
    }
    true
}

fn tac_deliver(owner: &TacActivation, index: usize, value: &TacValue, republish: bool) {
    let Some((_, TacMember::Listener(listener))) = owner.members.get(index) else {
        return;
    };
    match (listener.invoke)(value) {
        Ok(result) => {
            if republish {
                tac_republish(&result, listener.returns);
            }
        }
        Err(_) => tac_event_failure(),
    }
}

fn tac_retain(owner: &TacActivation, name: &str, value: &TacValue) -> bool {
    let mut retained = owner.retained.borrow_mut();
    if !retained.contains_key(name) && retained.len() == 32 {
        return false;
    }
    retained.insert(name.to_owned(), value.clone());
    true
}

fn tac_defer(owner: &TacActivation, index: usize, value: &TacValue) {
    let mut deferred = owner.deferred.borrow_mut();
    if deferred.len() >= 128 {
        tac_event_failure();
    } else {
        deferred.push((index, value.clone()));
    }
}

fn tac_notify_listeners(owner: &TacActivation, name: &str, value: &TacValue) {
    for index in &owner.listeners {
        let Some((_, TacMember::Listener(listener))) = owner.members.get(*index) else {
            continue;
        };
        if !listener.events.contains(&name) {
            continue;
        }
        if let Some(models) = listener.models {
            if !tac_fits(&TacScanner::new(models).value(), name, value) {
                tac_event_failure();
                continue;
            }
        }
        if let Some(condition) = listener.condition {
            if !tac_matches(&TacScanner::new(condition).value(), value, name) {
                continue;
            }
        }
        if listener.asynchronous {
            tac_defer(owner, *index, value);
            continue;
        }
        tac_deliver(owner, *index, value, true);
    }
}

fn tac_drain(owner: &TacActivation) {
    let mut delivered = 0;
    loop {
        if owner.disposed.get() {
            return;
        }
        let pending = core::mem::take(&mut *owner.deferred.borrow_mut());
        if !pending.is_empty() {
            for (index, value) in pending {
                tac_deliver(owner, index, &value, false);
            }
            continue;
        }
        let next = owner.queue.borrow_mut().pop_front();
        let Some((name, value, retain)) = next else {
            return;
        };
        if delivered == 128 {
            owner.queue.borrow_mut().clear();
            tac_event_failure();
            return;
        }
        delivered += 1;
        if cfg!(target_arch = "wasm32") {
            tac_send_event(
                &owner.route,
                &owner.service,
                &owner.owner,
                &name,
                &value,
                retain,
            );
            continue;
        }
        if retain && !tac_retain(owner, &name, &value) {
            tac_event_failure();
            continue;
        }
        tac_notify_listeners(owner, &name, &value);
        tac_send_event(
            &owner.route,
            &owner.service,
            &owner.owner,
            &name,
            &value,
            retain,
        );
    }
}

fn tac_dispose_activation(owner: std::rc::Rc<TacActivation>) {
    owner.disposed.set(true);
    owner.capabilities.dispose();
    owner.queue.borrow_mut().clear();
    owner.retained.borrow_mut().clear();
    owner.deferred.borrow_mut().clear();
    for (_, member) in &owner.members {
        if let TacMember::Hook(true, invoke) = member {
            if invoke().is_err() {
                tac_event_failure();
            }
        }
    }
}

fn tac_dispose_owner(id: &str) {
    let owner = tac_active_owners().with(|slot| slot.borrow_mut().remove(id));
    let is_executing = tac_executing().with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|executing| executing.owner == id)
    });
    if is_executing {
        tac_executing().with(|slot| slot.borrow_mut().take());
    }
    if let Some(owner) = owner {
        tac_dispose_activation(owner);
    }
    NativePageServices::dispose_owner(id);
}

fn tac_dispose_all() {
    let owners = tac_active_owners().with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    tac_executing().with(|slot| slot.borrow_mut().take());
    for (_, owner) in owners {
        tac_dispose_activation(owner);
    }
    NativePageServices::dispose_all();
}

fn tac_invoke_owned(raw: &str) -> String {
    if tac_handling().with(|handling| handling.replace(true)) {
        return String::from("{\"error\":\"Native owner is busy.\"}");
    }
    let answer = tac_handle(raw);
    let executing = tac_executing().with(|slot| slot.borrow().clone());
    if let Some(owner) = executing {
        tac_drain(&owner);
    }
    tac_executing().with(|slot| slot.borrow_mut().take());
    tac_handling().with(|handling| handling.set(false));
    answer
}
