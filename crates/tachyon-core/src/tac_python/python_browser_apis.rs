use crate::Failure;
use crate::web_api_clients::WebApiClients;
use std::ops::Range;
use std::path::Path;

pub(crate) struct PythonBrowserApis;

impl PythonBrowserApis {
    const EXTENSION: &'static str = "py";
    const NAME_JOINER: char = '_';
    const HELP: &'static str = "This browser API has no Python client. Use it in a JavaScript, TypeScript or compiled service, and pass the value to the Python service as an argument.";
    const SUPPORT: [&'static str; 3] = [
        include_str!("web_apis/python_web_host.py"),
        include_str!("web_apis/browser_fakes.py"),
        include_str!("web_apis/python_browser_clients.py"),
    ];
    const CLIENTS: &'static str = concat!(
        include_str!("web_apis/clock.py"),
        include_str!("web_apis/secure_random.py"),
        include_str!("web_apis/connectivity.py"),
        include_str!("web_apis/page_visibility.py"),
        include_str!("web_apis/preferences.py"),
        include_str!("web_apis/clipboard.py"),
        include_str!("web_apis/share.py"),
        include_str!("web_apis/notifications.py"),
        include_str!("web_apis/geolocation.py"),
        include_str!("web_apis/push.py"),
        include_str!("web_apis/window_controls.py"),
        include_str!("web_apis/focus.py"),
        include_str!("web_apis/files.py"),
    );
    const REGISTER: &'static str = "_TacPythonBrowserClients.register";

    pub(crate) fn source() -> String {
        let support = Self::SUPPORT.join("\n");
        let names = WebApiClients::TYPE_NAMES.join(", ");
        format!(
            "{support}\n{}\n{}({names})\n",
            Self::CLIENTS,
            Self::REGISTER
        )
    }

    pub(super) fn is_client(type_name: &str) -> bool {
        WebApiClients::TYPE_NAMES.contains(&type_name)
    }

    pub(super) fn refuse(path: &Path, source: &str) -> Result<(), Failure> {
        Self::first_use(source).map_or(Ok(()), |range| Err(Self::failure(path, source, range)))
    }

    fn failure(path: &Path, source: &str, range: Range<usize>) -> Failure {
        let name = &source[range.clone()];
        Failure::one(crate::failure::diagnostic(
            1605,
            format!("Python Tac services cannot use `{name}`: it belongs to the page."),
            Some(String::from(Self::HELP)),
            crate::failure::source_span(&path.to_string_lossy(), range.start, range.end),
        ))
    }

    fn first_use(source: &str) -> Option<Range<usize>> {
        let code = crate::lexical::code_mask(Self::EXTENSION, source).code;
        WebApiClients::client_type_names()
            .filter(|name| !Self::is_client(name))
            .flat_map(|name| {
                source
                    .match_indices(name)
                    .map(|(at, _)| at..at + name.len())
            })
            .filter(|range| Self::is_code_word(source, &code, range))
            .min_by_key(|range| range.start)
    }

    fn is_code_word(source: &str, code: &[bool], range: &Range<usize>) -> bool {
        let before = source[..range.start].chars().next_back();
        let after = source[range.end..].chars().next();
        [
            code.get(range.start).copied().unwrap_or(false),
            !Self::continues_name(before),
            !Self::continues_name(after),
        ] == [true, true, true]
    }

    fn continues_name(character: Option<char>) -> bool {
        character.is_some_and(|character| {
            [character.is_alphanumeric(), character == Self::NAME_JOINER].contains(&true)
        })
    }
}
