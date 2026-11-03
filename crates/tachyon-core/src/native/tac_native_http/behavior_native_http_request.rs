#[cfg(not(target_arch = "wasm32"))]
struct BehaviorNativeHttpRequest;

#[cfg(not(target_arch = "wasm32"))]
impl BehaviorNativeHttpRequest {
    const METHODS: [&'static str; 6] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"];
    const BODILESS: [&'static str; 2] = ["GET", "HEAD"];
    const REFUSED_HEADERS: [&'static str; 7] = [
        "host",
        "connection",
        "content-length",
        "transfer-encoding",
        "cookie",
        "origin",
        "referer",
    ];
    const REFUSED_PREFIXES: [&'static str; 2] = ["sec-", "proxy-"];
    const TOKEN_MARKS: &'static str = "!#$%&'*+.^_`|~-";
    const PATH_MARKS: &'static str = "\\?#%";
    const FORM_MARKS: &'static str = "*-._";
    const VALUE_BREAKS: [char; 3] = ['\r', '\n', '\0'];
    const MOST_HEADERS: usize = 128;
    const MOST_NAME: usize = 128;
    const MOST_TEXT: usize = 8192;
    const MOST_QUERY_KEY: usize = 256;
    const MOST_PATH: usize = 2048;
    const MOST_URL: usize = 8192;
    const MOST_BYTES: usize = 1_048_576;
    const LATEST_MS: u64 = 60_000;
    const SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    const REQUEST: &'static str = "HTTP_REQUEST";
    const ORIGIN: &'static str = "HTTP_ORIGIN";
    const HEADER: &'static str = "HTTP_HEADER";
    const QUERY: &'static str = "HTTP_QUERY";
    const BODY: &'static str = "HTTP_BODY";
    const LIMIT: &'static str = "HTTP_LIMIT";
    const MISSING: &'static str = "HTTP_MISSING";
    const JSON_TYPE: (&'static str, &'static str) = ("content-type", "application/json");

    fn read(text: &str) -> Result<TacNativeHttpRequest, &'static str> {
        let fields = Self::fields(text)?;
        let method = Self::method(&fields)?;
        Self::policy(&fields)?;
        let deadline = Self::deadline(&fields)?;
        let limit = Self::limit(&fields)?;
        let path = Self::path(&fields)?;
        let target = Self::target(path, fields.get("query"))?;
        let mut headers = Self::headers(fields.get("headers"))?;
        let body = Self::body(method, fields.get("body"), &mut headers)?;
        let origin = Self::origin()?;
        Ok(TacNativeHttpRequest {
            method,
            origin,
            target,
            headers,
            body,
            deadline,
            limit,
        })
    }

    fn fields(
        text: &str,
    ) -> Result<std::collections::BTreeMap<String, BehaviorValue>, &'static str> {
        let mut reader = BehaviorJsonReader { text, at: 0 };
        match reader.value(0) {
            Some(BehaviorValue::Object(fields)) => Ok(fields),
            _ => Err(Self::REQUEST),
        }
    }

    fn text<'value>(
        fields: &'value std::collections::BTreeMap<String, BehaviorValue>,
        key: &str,
    ) -> Option<&'value str> {
        match fields.get(key) {
            Some(BehaviorValue::String(text)) => Some(text.as_str()),
            _ => None,
        }
    }

    fn number(fields: &std::collections::BTreeMap<String, BehaviorValue>, key: &str) -> Option<f64> {
        match fields.get(key) {
            Some(BehaviorValue::Number(number)) => Some(*number),
            _ => None,
        }
    }

    fn method(
        fields: &std::collections::BTreeMap<String, BehaviorValue>,
    ) -> Result<&'static str, &'static str> {
        let named = Self::text(fields, "method");
        Self::METHODS
            .into_iter()
            .find(|method| named == Some(*method))
            .ok_or(Self::REQUEST)
    }

    fn policy(fields: &std::collections::BTreeMap<String, BehaviorValue>) -> Result<(), &'static str> {
        let declared = [Self::text(fields, "redirect"), Self::text(fields, "credentials")];
        (declared == [Some("error"), Some("omit")])
            .then_some(())
            .ok_or(Self::REQUEST)
    }

    fn deadline(fields: &std::collections::BTreeMap<String, BehaviorValue>) -> Result<u64, &'static str> {
        let latest = TacNativeHttp::now_ms().saturating_add(Self::LATEST_MS);
        Self::number(fields, "deadline")
            .filter(|deadline| Self::whole(*deadline))
            .map(|deadline| deadline as u64)
            .filter(|deadline| *deadline <= latest)
            .ok_or(Self::REQUEST)
    }

    fn limit(fields: &std::collections::BTreeMap<String, BehaviorValue>) -> Result<usize, &'static str> {
        Self::number(fields, "maxResponseBytes")
            .filter(|limit| Self::whole(*limit))
            .map(|limit| limit as usize)
            .filter(|limit| (1..=Self::MOST_BYTES).contains(limit))
            .ok_or(Self::LIMIT)
    }

    fn whole(number: f64) -> bool {
        [number.fract() == 0.0, (0.0..=Self::SAFE_INTEGER).contains(&number)] == [true, true]
    }

    fn path(fields: &std::collections::BTreeMap<String, BehaviorValue>) -> Result<&str, &'static str> {
        Self::text(fields, "path")
            .filter(|path| Self::plain_path(path))
            .ok_or(Self::ORIGIN)
    }

    fn plain_path(path: &str) -> bool {
        let rooted = [path.starts_with('/'), !path.starts_with("//")];
        let marks = path
            .chars()
            .any(|mark| mark.is_whitespace() | Self::PATH_MARKS.contains(mark));
        let moves = path.split('/').any(|part| [".", ".."].contains(&part));
        let checks = [rooted == [true, true], !marks, !moves, path.len() <= Self::MOST_PATH];
        checks == [true; 4]
    }

    fn origin() -> Result<&'static str, &'static str> {
        Some(TAC_YON_ORIGIN)
            .filter(|origin| !origin.is_empty())
            .ok_or(Self::MISSING)
    }

    fn target(path: &str, query: Option<&BehaviorValue>) -> Result<String, &'static str> {
        let pairs = Self::query(query)?;
        let encoded = pairs
            .iter()
            .map(|(key, value)| format!("{}={}", Self::form(key), Self::form(value)))
            .collect::<Vec<_>>()
            .join("&");
        let target = [path, encoded.as_str()].join(["", "?"][usize::from(!encoded.is_empty())]);
        (target.len() <= Self::MOST_URL)
            .then_some(target)
            .ok_or(Self::QUERY)
    }

    fn query(query: Option<&BehaviorValue>) -> Result<Vec<(String, String)>, &'static str> {
        match query {
            None | Some(BehaviorValue::Null) => Ok(Vec::new()),
            Some(BehaviorValue::Object(entries)) => entries
                .iter()
                .map(|(key, value)| Self::query_pair(key, value))
                .collect(),
            Some(_) => Err(Self::QUERY),
        }
    }

    fn query_pair(key: &str, value: &BehaviorValue) -> Result<(String, String), &'static str> {
        let written = Self::query_value(value)
            .filter(|written| written.len() <= Self::MOST_TEXT)
            .filter(|_| key.chars().count() <= Self::MOST_QUERY_KEY);
        written
            .map(|written| (key.to_owned(), written))
            .ok_or(Self::QUERY)
    }

    fn query_value(value: &BehaviorValue) -> Option<String> {
        match value {
            BehaviorValue::String(text) => Some(text.clone()),
            BehaviorValue::Bool(flag) => Some(flag.to_string()),
            BehaviorValue::Number(number) => Self::number_text(*number),
            _ => None,
        }
    }

    fn number_text(number: f64) -> Option<String> {
        let whole = [number.fract() == 0.0, number.abs() <= Self::SAFE_INTEGER] == [true, true];
        let written = [format!("{number}"), format!("{}", number as i64)][usize::from(whole)].clone();
        number.is_finite().then_some(written)
    }

    fn form(text: &str) -> String {
        text.bytes().map(Self::form_byte).collect()
    }

    fn form_byte(byte: u8) -> String {
        let kept = byte.is_ascii_alphanumeric() | Self::FORM_MARKS.as_bytes().contains(&byte);
        let space = byte == b' ';
        [format!("%{byte:02X}"), String::from(char::from(byte)), String::from("+")]
            [usize::from(kept) + 2 * usize::from(space)]
            .clone()
    }

    fn headers(values: Option<&BehaviorValue>) -> Result<Vec<(String, String)>, &'static str> {
        match values {
            None | Some(BehaviorValue::Null) => Ok(Vec::new()),
            Some(BehaviorValue::Object(entries)) if entries.len() <= Self::MOST_HEADERS => entries
                .iter()
                .map(|(name, value)| Self::header(name, value))
                .collect(),
            Some(_) => Err(Self::HEADER),
        }
    }

    fn header(name: &str, value: &BehaviorValue) -> Result<(String, String), &'static str> {
        Self::string(value)
            .filter(|text| Self::header_value(text))
            .filter(|_| Self::header_name(name))
            .map(|text| (name.to_ascii_lowercase(), text.to_owned()))
            .ok_or(Self::HEADER)
    }

    fn string(value: &BehaviorValue) -> Option<&str> {
        match value {
            BehaviorValue::String(text) => Some(text.as_str()),
            _ => None,
        }
    }

    fn header_name(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        let token = name
            .chars()
            .all(|mark| mark.is_ascii_alphanumeric() | Self::TOKEN_MARKS.contains(mark));
        let refused = Self::REFUSED_HEADERS.contains(&lower.as_str());
        let reserved = Self::REFUSED_PREFIXES
            .iter()
            .any(|prefix| lower.starts_with(prefix));
        let sized = (1..=Self::MOST_NAME).contains(&name.len());
        [token, !refused, !reserved, sized] == [true; 4]
    }

    fn header_value(text: &str) -> bool {
        let broken = text.contains(Self::VALUE_BREAKS);
        [!broken, text.len() <= Self::MOST_TEXT] == [true, true]
    }

    fn body(
        method: &'static str,
        body: Option<&BehaviorValue>,
        headers: &mut Vec<(String, String)>,
    ) -> Result<Option<String>, &'static str> {
        let Some(body) = body else {
            return Ok(None);
        };
        Self::require_body(method)?;
        headers.retain(|(name, _)| name != Self::JSON_TYPE.0);
        headers.push((Self::JSON_TYPE.0.to_owned(), Self::JSON_TYPE.1.to_owned()));
        body.to_json(Self::MOST_BYTES)
            .map(Some)
            .map_err(|_| Self::BODY)
    }

    fn require_body(method: &'static str) -> Result<(), &'static str> {
        (!Self::BODILESS.contains(&method))
            .then_some(())
            .ok_or(Self::BODY)
    }
}
