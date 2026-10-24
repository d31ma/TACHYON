use crate::events::{EventModule, EventStream, HeldStreams};
use crate::project::NativeCompanion;
use crate::template::RequiredFields;
use std::path::Path;

pub(super) struct KotlinHeldStreams;

impl KotlinHeldStreams {
    const PROPERTY_WORDS: [&'static str; 2] = ["val", "var"];
    const MODIFIERS: [&'static str; 8] = [
        "private",
        "public",
        "internal",
        "protected",
        "lateinit",
        "override",
        "open",
        "final",
    ];
    const TOO_MANY: &'static str = "A Kotlin service holds at most 16 @StreamRequest and @SocketRequest methods through its clients.";

    pub(super) fn of_owner(
        language: NativeCompanion,
        module: &EventModule,
        owner: Option<&str>,
        path: &Path,
    ) -> Result<String, &'static str> {
        let kotlin_owner = owner.filter(|_| language == NativeCompanion::Kotlin);
        kotlin_owner.map_or(Ok(String::new()), |owner| {
            Self::entries(module, owner, path)
        })
    }

    fn entries(module: &EventModule, owner: &str, path: &Path) -> Result<String, &'static str> {
        let field_types = Self::held_types(module, owner, path);
        let streams = HeldStreams::held(&HeldStreams::clients(module, owner), &field_types);
        let within_limit = streams.len() <= HeldStreams::MOST_STREAMS;
        within_limit
            .then(|| {
                streams
                    .into_iter()
                    .map(|(client, stream)| Self::entry(client, stream))
                    .collect()
            })
            .ok_or(Self::TOO_MANY)
    }

    fn held_types(module: &EventModule, owner: &str, path: &Path) -> Vec<String> {
        let code =
            crate::annotation_layout::masked_source(path, &module.source).unwrap_or_default();
        let body = RequiredFields::class_body(&code, owner).map_or("", |(_, body)| body);
        let mut depth = 0usize;
        let mut held = Vec::new();
        for line in body.lines() {
            held.extend((depth == 0).then(|| Self::property_type(line)).flatten());
            let opened = line.matches('{').count();
            let closed = line.matches('}').count();
            depth = (depth + opened).saturating_sub(closed);
        }
        held
    }

    fn property_type(line: &str) -> Option<String> {
        let words: Vec<&str> = line.split_whitespace().collect();
        let keyword = words
            .iter()
            .position(|word| Self::PROPERTY_WORDS.contains(word))?;
        let modified = words
            .iter()
            .take(keyword)
            .all(|word| Self::MODIFIERS.contains(word));
        let declared = words.get(keyword + 1..)?.join(" ");
        let (_, typed) = declared.split_once(':')?;
        let type_name = typed.split(['=', '{']).next()?.trim();
        modified.then(|| type_name.trim_end_matches('?').to_owned())
    }

    fn entry(client: &str, stream: &EventStream) -> String {
        let key = format!("__stream_{client}_{}", stream.method);
        let models = stream.models.as_ref().map_or_else(
            || String::from("null"),
            |models| Self::text(&models.to_string()),
        );
        format!(
            "\"{key}\" to TacStream({}, {}, {}, {}, {models}),\n",
            Self::text(client),
            Self::text(&stream.path),
            Self::text(&stream.event),
            stream.socket
        )
    }

    fn text(value: &str) -> String {
        super::events::string_literal(NativeCompanion::Kotlin, value)
    }
}
