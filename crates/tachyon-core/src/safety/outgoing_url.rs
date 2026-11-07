#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OutgoingUrl {
    pub(super) at: usize,
    pub(super) host: String,
}

impl OutgoingUrl {
    const SCHEME_MARK: &'static str = "://";
    const SCHEMES: [&'static str; 4] = ["https", "http", "wss", "ws"];
    const QUOTES: [char; 3] = ['"', '\'', '`'];
    const HOST_ENDS: [char; 12] = ['/', ':', '?', '#', '"', '\'', '`', '$', '{', '}', '\\', ' '];
    const SEARCH_BACK: usize = 400;
    const CALLEES: &'static str = "fetch WebSocket EventSource sendBeacon urlopen Request HttpRequestMessage URL URI Uri Url NewRequest NewRequestWithContext GetAsync PostAsync PutAsync PatchAsync DeleteAsync SendAsync GetStringAsync GetByteArrayAsync GetStreamAsync ConnectAsync file_get_contents curl_init getUrl postUrl openUrl";
    const ANY_ARGUMENT: &'static str = "NewRequest NewRequestWithContext HttpRequestMessage";
    const QUALIFIED_CALLEES: &'static str = "URI.create URI.parse URI.open Uri.parse Url::parse url.Parse http.Get http.Post http.Head requests.get requests.post requests.put requests.patch requests.delete requests.head requests.request httpx.get httpx.post httpx.put httpx.patch httpx.delete reqwest::get ureq::get ureq::post Net::HTTP.get Net::HTTP.post_form Net::HTTP.get_response";

    pub(super) fn find(text: &str, code: &str) -> Vec<Self> {
        let marks: Vec<usize> = text
            .match_indices(Self::SCHEME_MARK)
            .map(|(at, _)| at)
            .collect();
        marks
            .iter()
            .filter_map(|mark| Self::at_mark(text, code, *mark))
            .collect()
    }

    fn at_mark(text: &str, code: &str, mark: usize) -> Option<Self> {
        let quote = Self::literal_start(text, mark)?;
        Self::is_outgoing_call(code, quote).then_some(())?;
        let host = Self::host_after(&text[mark + Self::SCHEME_MARK.len()..]);
        Some(Self { at: quote, host })
    }

    fn literal_start(text: &str, mark: usize) -> Option<usize> {
        let scheme = Self::SCHEMES
            .iter()
            .find(|scheme| text[..mark].ends_with(**scheme))?;
        let start = mark - scheme.len();
        let before = text[..start].chars().next_back()?;
        Self::QUOTES
            .contains(&before)
            .then(|| start - before.len_utf8())
    }

    fn host_after(rest: &str) -> String {
        let end = rest.find(Self::HOST_ENDS).unwrap_or(rest.len());
        rest[..end].to_ascii_lowercase()
    }

    fn is_outgoing_call(code: &str, quote: usize) -> bool {
        let (callee, first) = Self::callee_before(code, quote);
        let last = callee.rsplit(['.', ':', '>']).next().unwrap_or("");
        let named = Self::CALLEES.split(' ').any(|name| name == last);
        let qualified = Self::QUALIFIED_CALLEES
            .split(' ')
            .any(|name| callee.ends_with(name));
        let anywhere = Self::ANY_ARGUMENT.split(' ').any(|name| name == last);
        let known = [named, qualified].contains(&true);
        let placed = [first, anywhere].contains(&true);
        [known, placed] == [true, true]
    }

    fn callee_before(code: &str, quote: usize) -> (String, bool) {
        let start = quote.saturating_sub(Self::SEARCH_BACK);
        let window = code.get(start..quote).unwrap_or("");
        let opening = Self::open_parenthesis(window);
        opening.map_or_else(
            || (String::new(), false),
            |at| {
                (
                    Self::name_before(&window[..at]),
                    !Self::has_comma(&window[at + 1..]),
                )
            },
        )
    }

    fn has_comma(arguments: &str) -> bool {
        let mut depth = 0_usize;
        arguments
            .chars()
            .any(|character| Self::top_level_comma(&mut depth, character))
    }

    fn top_level_comma(depth: &mut usize, character: char) -> bool {
        let comma = [character == ',', *depth == 0] == [true, true];
        *depth = Self::inner_depth(*depth, character);
        comma
    }

    fn inner_depth(depth: usize, character: char) -> usize {
        match character {
            '(' | '[' | '{' => depth + 1,
            ')' | ']' | '}' => depth.saturating_sub(1),
            _ => depth,
        }
    }

    fn open_parenthesis(window: &str) -> Option<usize> {
        let mut depth = 0_usize;
        window
            .char_indices()
            .rev()
            .find(|(_, character)| Self::closes_depth(&mut depth, *character))
            .map(|(at, _)| at)
    }

    fn closes_depth(depth: &mut usize, character: char) -> bool {
        let opens_here = [character == '(', *depth == 0] == [true, true];
        *depth = Self::next_depth(*depth, character);
        opens_here
    }

    fn next_depth(depth: usize, character: char) -> usize {
        match character {
            ')' => depth + 1,
            '(' => depth.saturating_sub(1),
            _ => depth,
        }
    }

    fn name_before(text: &str) -> String {
        let trimmed = text.trim_end();
        let start = trimmed
            .rfind(|character: char| !Self::path_character(character))
            .map_or(0, |at| at + 1);
        String::from(&trimmed[start..])
    }

    fn path_character(character: char) -> bool {
        matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.' | ':' | '$')
    }
}
