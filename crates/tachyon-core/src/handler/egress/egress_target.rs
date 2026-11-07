#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EgressTarget {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) tunnel: bool,
    pub(super) head: Vec<u8>,
}

impl EgressTarget {
    const CONNECT: &'static str = "CONNECT";
    const PLAIN_SCHEME: &'static str = "http://";
    const PLAIN_PORT: u16 = 80;
    const LINE_END: &'static str = "\r\n";
    const PROXY_HEADER: &'static str = "proxy-";

    pub(super) fn read(head: &str) -> Option<Self> {
        let (first, rest) = head.split_once(Self::LINE_END)?;
        let mut words = first.split(' ');
        let method = words.next()?;
        let target = words.next()?;
        let version = words.next()?;
        Self::from_parts(
            method,
            target,
            &format!("{version}{}{rest}", Self::LINE_END),
        )
    }

    fn from_parts(method: &str, target: &str, tail: &str) -> Option<Self> {
        if method == Self::CONNECT {
            return Self::tunnel(target);
        }
        Self::plain(method, target, tail)
    }

    fn tunnel(authority: &str) -> Option<Self> {
        let (host, port) = Self::authority(authority)?;
        Some(Self {
            host,
            port: port?,
            tunnel: true,
            head: Vec::new(),
        })
    }

    fn plain(method: &str, target: &str, tail: &str) -> Option<Self> {
        let rest = target.strip_prefix(Self::PLAIN_SCHEME)?;
        let split = rest.find('/').unwrap_or(rest.len());
        let (authority, path) = rest.split_at(split);
        let (host, port) = Self::authority(authority)?;
        let head = Self::origin_head(method, Self::path_or_root(path), tail);
        Some(Self {
            host,
            port: port.unwrap_or(Self::PLAIN_PORT),
            tunnel: false,
            head: head.into_bytes(),
        })
    }

    fn path_or_root(path: &str) -> &str {
        if path.is_empty() {
            return "/";
        }
        path
    }

    fn origin_head(method: &str, path: &str, tail: &str) -> String {
        let lines: Vec<&str> = tail
            .split(Self::LINE_END)
            .filter(|line| !line.to_ascii_lowercase().starts_with(Self::PROXY_HEADER))
            .collect();
        format!("{method} {path} {}", lines.join(Self::LINE_END))
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
