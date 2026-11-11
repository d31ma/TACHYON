use tokio::io::{AsyncRead, AsyncReadExt};

pub(super) struct EgressHead;

impl EgressHead {
    const END: &'static [u8] = b"\r\n\r\n";
    const LINE_END: &'static str = "\r\n";
    const READ_CHUNK: usize = 4096;
    const PROXY_PREFIX: &'static str = "proxy-";
    const SEPARATOR: char = ':';
    const TOKEN_MARKS: &'static [u8] = b"!#$%&'*+-.^_`|~";

    pub(super) async fn read(
        reader: &mut (impl AsyncRead + Unpin),
        bytes: Vec<u8>,
        limit: usize,
    ) -> Option<(String, Vec<u8>)> {
        let bytes = Self::fill(reader, bytes, limit).await?;
        Self::split(bytes)
    }

    pub(super) fn rewrite(head: &str, dropped: &[&str], added: &[String]) -> Option<String> {
        let (first, fields) = head.split_once(Self::LINE_END)?;
        Self::rebuild(first, fields, dropped, added)
    }

    pub(super) fn rebuild(
        first: &str,
        fields: &str,
        dropped: &[&str],
        added: &[String],
    ) -> Option<String> {
        let lines = Self::lines(fields)?;
        let kept = Self::kept(lines, dropped);
        Some(Self::join(first, &kept, added))
    }

    pub(super) fn values(fields: &str, name: &str) -> Option<Vec<String>> {
        let lines = Self::lines(fields)?;
        Some(Self::matching(&lines, name))
    }

    async fn fill(
        reader: &mut (impl AsyncRead + Unpin),
        mut bytes: Vec<u8>,
        limit: usize,
    ) -> Option<Vec<u8>> {
        let mut chunk = vec![0_u8; Self::READ_CHUNK];
        while Self::end(&bytes).is_none() {
            let read = reader
                .read(&mut chunk)
                .await
                .ok()
                .filter(|read| *read > 0)?;
            bytes.extend_from_slice(&chunk[..read]);
            (bytes.len() <= limit).then_some(())?;
        }
        Some(bytes)
    }

    fn end(bytes: &[u8]) -> Option<usize> {
        bytes
            .windows(Self::END.len())
            .position(|window| window == Self::END)
    }

    fn split(mut bytes: Vec<u8>) -> Option<(String, Vec<u8>)> {
        let end = Self::end(&bytes)? + Self::END.len();
        let rest = bytes.split_off(end);
        let head = String::from_utf8(bytes).ok()?;
        Some((head, rest))
    }

    fn lines(fields: &str) -> Option<Vec<(String, &str, &str)>> {
        fields
            .split(Self::LINE_END)
            .take_while(|line| !line.is_empty())
            .map(Self::named)
            .collect()
    }

    fn named(line: &str) -> Option<(String, &str, &str)> {
        let (name, value) = line.split_once(Self::SEPARATOR)?;
        Self::is_token(name).then(|| (name.to_ascii_lowercase(), value.trim(), line))
    }

    fn is_token(name: &str) -> bool {
        !name.is_empty() && name.bytes().all(Self::is_token_byte)
    }

    fn is_token_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || Self::TOKEN_MARKS.contains(&byte)
    }

    fn kept<'a>(lines: Vec<(String, &'a str, &'a str)>, dropped: &[&str]) -> Vec<&'a str> {
        lines
            .into_iter()
            .filter(|(name, _, _)| Self::keeps(name, dropped))
            .map(|(_, _, line)| line)
            .collect()
    }

    fn keeps(name: &str, dropped: &[&str]) -> bool {
        !(dropped.contains(&name) || name.starts_with(Self::PROXY_PREFIX))
    }

    fn matching(lines: &[(String, &str, &str)], name: &str) -> Vec<String> {
        lines
            .iter()
            .filter(|(field, _, _)| field == name)
            .map(|(_, value, _)| (*value).to_owned())
            .collect()
    }

    fn join(first: &str, kept: &[&str], added: &[String]) -> String {
        let mut head = format!("{first}{}", Self::LINE_END);
        for line in kept.iter().copied().chain(added.iter().map(String::as_str)) {
            head.push_str(line);
            head.push_str(Self::LINE_END);
        }
        head.push_str(Self::LINE_END);
        head
    }
}
