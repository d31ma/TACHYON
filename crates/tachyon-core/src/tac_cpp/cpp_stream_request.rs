use super::{CppClass, CppFailure, CppMember, CppMemberKind, CppTypeText};
use crate::Failure;
use std::path::Path;

pub(crate) struct CppStreamRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) event: String,
    pub(crate) socket: bool,
}

impl CppStreamRequest {
    const MARKERS: [&'static str; 2] = ["StreamRequest", "SocketRequest"];
    const SOCKET: &'static str = "SocketRequest";
    const FORM: &'static str = "A C++ [[StreamRequest]] or [[SocketRequest]] method returns void, takes the message model as its one parameter and has no body, for example `[[StreamRequest]] void receive(const Tick& tick);`. Tachyon holds the stream or socket in the page and publishes each message as that model's event for [[OnEvent]]";
    const PATH_FORM: &'static str = "[[StreamRequest]] and [[SocketRequest]] take an optional path in quotes that starts with /, for example [[StreamRequest(\"/ticks\")]]. It is added to the [[HTTPClient]] path";
    const ONE_SOCKET: &'static str = "A C++ [[HTTPClient]] class has at most one [[SocketRequest]] method, because its send(message) method writes to that one socket";
    const SEND_NAME: &'static str = "A C++ [[HTTPClient]] class with a [[SocketRequest]] method gets a send(message) method from Tachyon, so none of its own methods may be named send";
    const SEND: &'static str = " public: template <typename Message> tachyon::Task<void> send(Message message) const { return tachyon::http::TacSocket::send(\"{client}\", std::move(message)); }";

    pub(crate) fn is_stream(member: &CppMember) -> bool {
        Self::MARKERS.iter().any(|marker| member.marked(marker))
    }

    pub(crate) fn check(path: &Path, source: &str, class: &CppClass) -> Result<(), Failure> {
        let streams: Vec<&CppMember> = class
            .members
            .iter()
            .filter(|member| Self::is_stream(member))
            .collect();
        streams
            .iter()
            .try_for_each(|member| Self::check_member(path, source, member))?;
        Self::check_one_socket(path, &streams)?;
        Self::check_send_name(path, class, &streams)
    }

    pub(crate) fn read_all(class: &CppClass, base: &str) -> Vec<Self> {
        class
            .members
            .iter()
            .filter(|member| Self::is_stream(member))
            .map(|member| Self::read(member, base))
            .collect()
    }

    pub(crate) fn send_member(class: &CppClass) -> String {
        let socket = class
            .members
            .iter()
            .any(|member| member.marked(Self::SOCKET));
        let sends: [&str; 2] = ["", Self::SEND];
        sends[usize::from(socket)].replace("{client}", &class.name)
    }

    fn read(member: &CppMember, base: &str) -> Self {
        let own = Self::own_path(member).unwrap_or_default();
        let full = format!("{base}{own}");
        let parameter = member.parameters.first();
        Self {
            method: member.name.clone(),
            path: [full.as_str(), "/"][usize::from(full.is_empty())].to_owned(),
            event: parameter.map_or_else(String::new, |parameter| {
                CppTypeText::plain(&parameter.type_text)
            }),
            socket: member.marked(Self::SOCKET),
        }
    }

    fn check_member(path: &Path, source: &str, member: &CppMember) -> Result<(), Failure> {
        let markers = Self::MARKERS
            .iter()
            .filter(|marker| member.marked(marker))
            .count();
        let fits = [
            member.kind == CppMemberKind::Method,
            markers == 1,
            member.plain_type() == "void",
            member.parameters.len() == 1,
            source.as_bytes().get(member.terminator) != Some(&b'{'),
        ] == [true, true, true, true, true];
        fits.then_some(())
            .ok_or_else(|| CppFailure::unreadable(path, member.name_at, Self::FORM))?;
        Self::own_path(member)
            .map(drop)
            .ok_or_else(|| CppFailure::unreadable(path, member.name_at, Self::PATH_FORM))
    }

    fn own_path(member: &CppMember) -> Option<String> {
        let written = member
            .markers
            .iter()
            .find(|marker| Self::MARKERS.contains(&marker.name.as_str()))
            .map_or("", |marker| marker.arguments.trim());
        let quoted = serde_json::from_str::<String>(written)
            .ok()
            .filter(|own| own.starts_with('/'));
        [quoted, Some(String::new())][usize::from(written.is_empty())].clone()
    }

    fn check_one_socket(path: &Path, streams: &[&CppMember]) -> Result<(), Failure> {
        let sockets: Vec<&&CppMember> = streams
            .iter()
            .filter(|member| member.marked(Self::SOCKET))
            .collect();
        let second = sockets.get(1).map_or(0, |member| member.name_at);
        (sockets.len() < 2)
            .then_some(())
            .ok_or_else(|| CppFailure::unreadable(path, second, Self::ONE_SOCKET))
    }

    fn check_send_name(
        path: &Path,
        class: &CppClass,
        streams: &[&CppMember],
    ) -> Result<(), Failure> {
        let socket = streams.iter().any(|member| member.marked(Self::SOCKET));
        let named = class.members.iter().find(|member| member.name == "send");
        let refused = named.filter(|_| socket).map(|member| member.name_at);
        refused.map_or(Ok(()), |at| {
            Err(CppFailure::unreadable(path, at, Self::SEND_NAME))
        })
    }
}
