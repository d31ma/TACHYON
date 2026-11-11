use super::{EgressFraming, EgressHead};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EgressTarget {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) tunnel: bool,
    pub(super) head: Vec<u8>,
    pub(super) framing: EgressFraming,
}

impl EgressTarget {
    const CONNECT: &'static str = "CONNECT";
    const PLAIN_SCHEME: &'static str = "http://";
    const PLAIN_PORT: u16 = 80;
    const LINE_END: &'static str = "\r\n";
    const DROPPED: [&'static str; 4] = ["connection", "keep-alive", "upgrade", "host"];
    const CLOSE: &'static str = "Connection: close";

    pub(super) fn read(head: &str) -> Option<Self> {
        let (first, fields) = head.split_once(Self::LINE_END)?;
        let mut words = first.split(' ');
        let method = words.next()?;
        let target = words.next()?;
        let version = words.next()?;
        Self::from_parts(method, target, version, fields)
    }

    fn from_parts(method: &str, target: &str, version: &str, fields: &str) -> Option<Self> {
        if method == Self::CONNECT {
            return Self::tunnel(target);
        }
        Self::plain(method, target, version, fields)
    }

    fn tunnel(authority: &str) -> Option<Self> {
        let (host, port) = Self::authority(authority)?;
        Some(Self {
            host,
            port: port?,
            tunnel: true,
            head: Vec::new(),
            framing: EgressFraming::Length(0),
        })
    }

    fn plain(method: &str, target: &str, version: &str, fields: &str) -> Option<Self> {
        let rest = target.strip_prefix(Self::PLAIN_SCHEME)?;
        let split = rest.find('/').unwrap_or(rest.len());
        let (authority, path) = rest.split_at(split);
        let (host, port) = Self::authority(authority)?;
        let first = format!("{method} {} {version}", Self::path_or_root(path));
        let head = Self::origin_head(&first, authority, fields)?;
        Some(Self {
            host,
            port: port.unwrap_or(Self::PLAIN_PORT),
            tunnel: false,
            head: head.into_bytes(),
            framing: EgressFraming::read(fields)?,
        })
    }

    fn path_or_root(path: &str) -> &str {
        if path.is_empty() {
            return "/";
        }
        path
    }

    fn origin_head(first: &str, authority: &str, fields: &str) -> Option<String> {
        let added = [format!("Host: {authority}"), String::from(Self::CLOSE)];
        EgressHead::rebuild(first, fields, &Self::DROPPED, &added)
    }

    fn authority(authority: &str) -> Option<(String, Option<u16>)> {
        let (host, port) = Self::host_and_port(authority);
        let port = port.map(str::parse::<u16>).transpose().ok()?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        (!host.is_empty()).then(|| (host.to_ascii_lowercase(), port))
    }

    fn host_and_port(authority: &str) -> (&str, Option<&str>) {
        let split = authority
            .rsplit_once(':')
            .filter(|(_, port)| !port.contains(']'));
        split.map_or((authority, None), |(host, port)| (host, Some(port)))
    }
}
