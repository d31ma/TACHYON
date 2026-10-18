struct BehaviorHostCache {
    name: String,
}
struct BehaviorHostEntities;
struct BehaviorHost;
impl BehaviorHost {
    const MAX_DEPTH: usize = 64;

    fn available() -> bool {
        __TACHYON_BEHAVIOR_ENVIRONMENT == "tac" && cfg!(target_arch = "wasm32")
    }

    fn ask(request: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let text = request.to_json(1_048_576)?;
        let answer = behavior_storage_host(&text)
            .ok_or_else(|| behavior_error("storage", "Tachyon storage is not on this page."))?;
        let mut reader = BehaviorJsonReader {
            text: &answer,
            at: 0,
        };
        let parsed = reader.value(0);
        let Some(BehaviorValue::Object(mut object)) = parsed else {
            return Err(behavior_error("storage", "Tachyon storage did not answer."));
        };
        if let Some(BehaviorValue::String(message)) = object.remove("error") {
            return Err(BehaviorError {
                code: "storage",
                message: "Tachyon storage refused the request.",
                path: Some(message),
                violations: Vec::new(),
            });
        }
        Ok(object.remove("value").unwrap_or(BehaviorValue::Null))
    }

    fn optional(value: Option<&BehaviorValue>) -> BehaviorValue {
        value.cloned().unwrap_or(BehaviorValue::Null)
    }
}
fn behavior_builtin_cache(name: &str) -> Option<std::sync::Arc<dyn BehaviorCacheAdapter>> {
    if !BehaviorHost::available() {
        return None;
    }
    Some(std::sync::Arc::new(BehaviorHostCache {
        name: name.to_owned(),
    }))
}
fn behavior_builtin_entities(name: &str) -> Option<std::sync::Arc<dyn BehaviorEntityAdapter>> {
    if name != "indexeddb" || !BehaviorHost::available() {
        return None;
    }
    Some(std::sync::Arc::new(BehaviorHostEntities))
}
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "tachyon")]
unsafe extern "C" {
    fn tac_host_storage(request: *const u8, length: usize) -> *mut u8;
}
#[cfg(target_arch = "wasm32")]
fn behavior_storage_host(request: &str) -> Option<String> {
    let answer = unsafe { tac_host_storage(request.as_ptr(), request.len()) };
    if answer.is_null() {
        return None;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(answer.cast()) }.to_bytes_with_nul();
    let length = bytes.len();
    let text = String::from_utf8_lossy(&bytes[..length - 1]).into_owned();
    drop(unsafe { Vec::from_raw_parts(answer, 0, length) });
    Some(text)
}
#[cfg(not(target_arch = "wasm32"))]
fn behavior_storage_host(request: &str) -> Option<String> {
    let _ = request;
    None
}
impl BehaviorCacheAdapter for BehaviorHostCache {
    fn get(
        &self,
        key: &str,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<Option<BehaviorCacheEntry>> {
        let _ = execution;
        let answer = BehaviorHost::ask(BehaviorValue::object([
            ("op", BehaviorValue::from("cache.get")),
            ("cache", BehaviorValue::from(self.name.as_str())),
            ("key", BehaviorValue::from(key)),
        ]))?;
        let BehaviorValue::Object(mut entry) = answer else {
            return Ok(None);
        };
        if entry.get("hit") != Some(&BehaviorValue::Bool(true)) {
            return Ok(None);
        }
        let expires_at = match entry.get("expiresAt") {
            Some(BehaviorValue::Number(value)) if *value >= 0.0 => *value as u64,
            _ => 0,
        };
        Ok(Some(BehaviorCacheEntry {
            value: entry.remove("value").unwrap_or(BehaviorValue::Null),
            expires_at,
        }))
    }

    fn set(
        &self,
        key: &str,
        value: BehaviorValue,
        write: &BehaviorCacheWrite,
    ) -> BehaviorResult<()> {
        BehaviorHost::ask(BehaviorValue::object([
            ("op", BehaviorValue::from("cache.set")),
            ("cache", BehaviorValue::from(self.name.as_str())),
            ("key", BehaviorValue::from(key)),
            ("value", value),
            ("expiresAt", BehaviorValue::Number(write.expires_at as f64)),
            (
                "maxEntries",
                BehaviorValue::Number(write.policy.max_entries as f64),
            ),
        ]))
        .map(|_| ())
    }

    fn delete(&self, key: &str, execution: &BehaviorExecution) -> BehaviorResult<()> {
        let _ = execution;
        BehaviorHost::ask(BehaviorValue::object([
            ("op", BehaviorValue::from("cache.delete")),
            ("cache", BehaviorValue::from(self.name.as_str())),
            ("key", BehaviorValue::from(key)),
        ]))
        .map(|_| ())
    }
}
impl BehaviorEntityAdapter for BehaviorHostEntities {
    fn execute(
        &self,
        command: &BehaviorEntityCommand,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue> {
        let _ = execution;
        let mut sent = std::collections::BTreeMap::new();
        sent.insert(String::from("entity"), BehaviorValue::from(command.entity));
        sent.insert(String::from("table"), BehaviorValue::from(command.table));
        sent.insert(String::from("idField"), BehaviorValue::from(command.id_field));
        sent.insert(String::from("operation"), BehaviorValue::from(command.operation));
        sent.insert(String::from("value"), BehaviorHost::optional(command.value.as_ref()));
        sent.insert(String::from("page"), BehaviorHost::optional(command.page.as_ref()));
        sent.insert(String::from("id"), BehaviorHost::optional(command.id.as_ref()));
        sent.insert(String::from("query"), BehaviorHost::optional(command.query.as_ref()));
        if let Some(transaction) = &command.transaction {
            sent.insert(
                String::from("transaction"),
                BehaviorValue::from(transaction.as_str()),
            );
        }
        BehaviorHost::ask(BehaviorValue::object([
            ("op", BehaviorValue::from("entity")),
            ("command", BehaviorValue::Object(sent)),
        ]))
    }

    fn transaction(
        &self,
        command: &BehaviorTransactionCommand,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue> {
        let _ = execution;
        let mut sent = std::collections::BTreeMap::new();
        sent.insert(String::from("operation"), BehaviorValue::from("transaction"));
        sent.insert(String::from("action"), BehaviorValue::from(command.action));
        if let Some(transaction) = &command.transaction {
            sent.insert(
                String::from("transaction"),
                BehaviorValue::from(transaction.as_str()),
            );
        }
        let answer = BehaviorHost::ask(BehaviorValue::object([
            ("op", BehaviorValue::from("entity")),
            ("command", BehaviorValue::Object(sent)),
        ]))?;
        match answer {
            BehaviorValue::Object(mut begun) if command.action == "begin" => {
                Ok(begun.remove("transaction").unwrap_or(BehaviorValue::Null))
            }
            other => Ok(other),
        }
    }
}
struct BehaviorJsonReader<'text> {
    text: &'text str,
    at: usize,
}
impl BehaviorJsonReader<'_> {
    fn byte(&self, at: usize) -> Option<u8> {
        self.text.as_bytes().get(at).copied()
    }

    fn skip_space(&mut self) {
        while self
            .byte(self.at)
            .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
    }

    fn value(&mut self, depth: usize) -> Option<BehaviorValue> {
        if depth > BehaviorHost::MAX_DEPTH {
            return None;
        }
        self.skip_space();
        match self.byte(self.at)? {
            b'{' => self.object(depth),
            b'[' => self.list(depth),
            b'"' => self.string().map(BehaviorValue::String),
            b't' => self.word("true", BehaviorValue::Bool(true)),
            b'f' => self.word("false", BehaviorValue::Bool(false)),
            b'n' => self.word("null", BehaviorValue::Null),
            _ => self.number(),
        }
    }

    fn word(&mut self, word: &str, value: BehaviorValue) -> Option<BehaviorValue> {
        let end = self.at + word.len();
        if self.text.get(self.at..end)? != word {
            return None;
        }
        self.at = end;
        Some(value)
    }

    fn number(&mut self) -> Option<BehaviorValue> {
        let start = self.at;
        while self
            .byte(self.at)
            .is_some_and(|byte| matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'))
        {
            self.at += 1;
        }
        self.text[start..self.at]
            .parse::<f64>()
            .ok()
            .map(BehaviorValue::Number)
    }

    fn list(&mut self, depth: usize) -> Option<BehaviorValue> {
        self.at += 1;
        let mut values = Vec::new();
        self.skip_space();
        if self.byte(self.at) == Some(b']') {
            self.at += 1;
            return Some(BehaviorValue::Array(values));
        }
        loop {
            values.push(self.value(depth + 1)?);
            self.skip_space();
            match self.byte(self.at)? {
                b',' => self.at += 1,
                b']' => {
                    self.at += 1;
                    return Some(BehaviorValue::Array(values));
                }
                _ => return None,
            }
        }
    }

    fn object(&mut self, depth: usize) -> Option<BehaviorValue> {
        self.at += 1;
        let mut entries = std::collections::BTreeMap::new();
        self.skip_space();
        if self.byte(self.at) == Some(b'}') {
            self.at += 1;
            return Some(BehaviorValue::Object(entries));
        }
        loop {
            self.skip_space();
            if self.byte(self.at) != Some(b'"') {
                return None;
            }
            let key = self.string()?;
            self.skip_space();
            if self.byte(self.at) != Some(b':') {
                return None;
            }
            self.at += 1;
            entries.insert(key, self.value(depth + 1)?);
            self.skip_space();
            match self.byte(self.at)? {
                b',' => self.at += 1,
                b'}' => {
                    self.at += 1;
                    return Some(BehaviorValue::Object(entries));
                }
                _ => return None,
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        self.at += 1;
        let mut units: Vec<u16> = Vec::new();
        let mut text = String::new();
        loop {
            let byte = self.byte(self.at)?;
            if byte != b'\\' || self.byte(self.at + 1) != Some(b'u') {
                text.push_str(&String::from_utf16(&units).ok()?);
                units.clear();
            }
            match byte {
                b'"' => {
                    self.at += 1;
                    return Some(text);
                }
                b'\\' => {
                    let escaped = self.byte(self.at + 1)?;
                    self.at += 2;
                    match escaped {
                        b'u' => {
                            let digits = self.text.get(self.at..self.at + 4)?;
                            units.push(u16::from_str_radix(digits, 16).ok()?);
                            self.at += 4;
                        }
                        b'n' => text.push('\n'),
                        b'r' => text.push('\r'),
                        b't' => text.push('\t'),
                        b'b' => text.push('\u{8}'),
                        b'f' => text.push('\u{c}'),
                        b'"' | b'\\' | b'/' => text.push(char::from(escaped)),
                        _ => return None,
                    }
                }
                _ => {
                    let character = self.text.get(self.at..)?.chars().next()?;
                    text.push(character);
                    self.at += character.len_utf8();
                }
            }
        }
    }
}
