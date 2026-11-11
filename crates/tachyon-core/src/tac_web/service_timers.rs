use crate::Failure;
use crate::events::{EventModule, EventTimer};
use crate::failure::diagnostic;
use crate::fsharp_source::FsharpView;
use crate::lua_source::LuaView;
use crate::tac_cpp::{CppFile, CppMember};
use serde_json::{Value, json};
use std::ffi::OsStr;
use std::path::Path;

pub(crate) struct ServiceTimers;

impl ServiceTimers {
    const MARKER_TEXT: &'static str = "Every";
    const UNREADABLE_SOURCE: u16 = 1605;
    const REALM: &'static str = "tac";
    const SHARED_READER: (
        &'static str,
        fn(&Path, &str, &str) -> Result<Vec<EventTimer>, Failure>,
    ) = ("", Self::shared_timers);
    const READERS: [(
        &'static str,
        fn(&Path, &str, &str) -> Result<Vec<EventTimer>, Failure>,
    ); 3] = [
        ("fs", Self::fsharp_timers),
        ("lua", Self::lua_timers),
        ("hpp", Self::cpp_timers),
    ];

    pub(crate) fn read(
        snapshot_root: &Path,
        source: &Path,
        class: &str,
    ) -> Result<Vec<EventTimer>, Failure> {
        let path = snapshot_root.join(source);
        let text =
            std::fs::read_to_string(&path).map_err(|error| Self::unreadable(&path, &error))?;
        let reader = Self::reader(&path, &text);
        reader(&path, &text, class)
    }

    pub(crate) fn described(timers: &[EventTimer]) -> Value {
        Value::Array(timers.iter().map(Self::entry).collect())
    }

    fn reader(
        path: &Path,
        text: &str,
    ) -> fn(&Path, &str, &str) -> Result<Vec<EventTimer>, Failure> {
        if !text.contains(Self::MARKER_TEXT) {
            return Self::no_timers;
        }
        Self::by_extension(path)
    }

    fn by_extension(path: &Path) -> fn(&Path, &str, &str) -> Result<Vec<EventTimer>, Failure> {
        let extension = path.extension().and_then(OsStr::to_str).unwrap_or_default();
        Self::READERS
            .iter()
            .find(|(name, _)| *name == extension)
            .unwrap_or(&Self::SHARED_READER)
            .1
    }

    fn no_timers(_path: &Path, _text: &str, _class: &str) -> Result<Vec<EventTimer>, Failure> {
        Ok(Vec::new())
    }

    fn shared_timers(path: &Path, text: &str, class: &str) -> Result<Vec<EventTimer>, Failure> {
        let module = crate::events::extract(path, text, Self::REALM)?;
        Ok(Self::owned(module, class))
    }

    fn fsharp_timers(path: &Path, text: &str, class: &str) -> Result<Vec<EventTimer>, Failure> {
        let view = FsharpView::tac(path, text);
        Self::shared_timers(view.path(), view.text(), class)
    }

    fn lua_timers(path: &Path, text: &str, class: &str) -> Result<Vec<EventTimer>, Failure> {
        let view = LuaView::joined(path, text);
        Self::shared_timers(view.path(), view.text(), class)
    }

    fn owned(module: Option<EventModule>, class: &str) -> Vec<EventTimer> {
        module
            .into_iter()
            .flat_map(|module| module.types)
            .filter(|event_type| event_type.name == class)
            .flat_map(|event_type| event_type.timers)
            .collect()
    }

    fn cpp_timers(path: &Path, text: &str, _class: &str) -> Result<Vec<EventTimer>, Failure> {
        let file = CppFile::read(path, text)?;
        let timers = file
            .class
            .members
            .iter()
            .filter(|member| member.marked(EventTimer::MARKER))
            .map(|member| Self::cpp_timer(path, member))
            .collect::<Result<Vec<_>, _>>()?;
        EventTimer::check_count(path, timers.len())?;
        Ok(timers)
    }

    fn cpp_timer(path: &Path, member: &CppMember) -> Result<EventTimer, Failure> {
        let marker = member
            .markers
            .iter()
            .find(|marker| marker.name == EventTimer::MARKER);
        let arguments = marker
            .map(|marker| marker.arguments.as_str())
            .unwrap_or_default();
        EventTimer::checked(path, &member.name, member.parameters.len(), arguments)
    }

    fn entry(timer: &EventTimer) -> Value {
        json!({ "name": timer.method, "every": timer.every })
    }

    fn unreadable(path: &Path, error: &std::io::Error) -> Failure {
        let message = format!(
            "{}: the service source cannot be read: {error}",
            path.display()
        );
        Failure::one(diagnostic(Self::UNREADABLE_SOURCE, &message, None, None))
    }
}
