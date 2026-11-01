use super::shape;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NameRefusal {
    NotSecret,
    Reserved,
    Segments,
}

impl NameRefusal {
    const MAX_SEGMENTS: usize = 16;
    const MAX_NAME_BYTES: usize = 128;
    const PREFIX: &'static str = "YON_";
    const RESERVED: [&'static str; 14] = [
        "YON_PROFILE",
        "YON_HOST",
        "YON_HOSTNAME",
        "YON_PORT",
        "YON_ISOLATION",
        "YON_SKIP_BUNDLE",
        "YON_APPLICATION_JSON",
        "YON_RESOURCE_ENDPOINT",
        "YON_RESOURCE_TOKEN",
        "YON_HANDLER_PROTOCOL",
        "YON_JAVASCRIPT_RUNTIME",
        "YON_PYTHON_RUNTIME",
        "YON_RUBY_RUNTIME",
        "YON_SERVER",
    ];
    const RESERVED_PREFIXES: [&'static str; 2] = ["YON_FIRECRACKER_", "YON_SERVER_"];
    const NAME_BYTES: &'static [u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

    pub(crate) fn of(name: &str) -> Option<Self> {
        let checks = [
            (!Self::is_secret_name(name), Self::NotSecret),
            (Self::is_reserved_name(name), Self::Reserved),
            (!Self::has_secret_segments(name), Self::Segments),
        ];
        checks
            .into_iter()
            .find(|(refused, _)| *refused)
            .map(|(_, refusal)| refusal)
    }

    pub(super) fn segments(name: &str) -> Vec<String> {
        name.split('_').map(str::to_ascii_lowercase).collect()
    }

    pub(super) const fn message(self) -> &'static str {
        Self::messages()[self as usize]
    }

    const fn messages() -> [&'static str; 3] {
        [
            "Project-root .env accepts only uppercase YON_ secret names; TAC_ secrets and non-prefixed names are forbidden.",
            "Project-root .env cannot set framework transport, profile, endpoint, runtime, or isolation policy names.",
            "Secret names require nonempty underscore-separated segments beginning with ASCII letters, with at most 16 segments.",
        ]
    }

    fn has_secret_segments(name: &str) -> bool {
        let segments = Self::segments(name);
        let shaped = segments
            .iter()
            .all(|segment| shape::is_path_segment(segment));
        [segments.len() <= Self::MAX_SEGMENTS, shaped] == [true, true]
    }

    fn is_secret_name(name: &str) -> bool {
        let allowed_bytes = name.bytes().all(|byte| Self::NAME_BYTES.contains(&byte));
        let short = name.len() <= Self::MAX_NAME_BYTES;
        [name.starts_with(Self::PREFIX), short, allowed_bytes] == [true; 3]
    }

    fn is_reserved_name(name: &str) -> bool {
        let prefixed = Self::RESERVED_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix));
        [Self::RESERVED.contains(&name), prefixed].contains(&true)
    }
}
