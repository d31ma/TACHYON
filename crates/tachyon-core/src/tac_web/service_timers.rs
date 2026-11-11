use crate::Failure;
use crate::events::{EventModule, EventTimer};
use crate::failure::diagnostic;
use crate::fsharp_source::FsharpView;
use crate::lua_source::LuaView;
use crate::tac_cpp::{CppFile, CppMember};
use serde_json::{Value, json};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(crate) struct ServiceTimers;

impl ServiceTimers {
    const MARKER_WORD: &'static str = "every";
    const UNREADABLE_SOURCE: u16 = 1605;
    const REALM: &'static str = "tac";
    const FSHARP: &'static str = "fs";
    const LUA: &'static str = "lua";
    const CPP: &'static str = "hpp";

    pub(crate) fn read(
        snapshot_root: &Path,
        source: &Path,
        class: &str,
    ) -> Result<Vec<EventTimer>, Failure> {
        let path = snapshot_root.join(source);
        let text =
            std::fs::read_to_string(&path).map_err(|error| Self::unreadable(&path, &error))?;
        let marked = Self::marked(&text);
        let shared = Self::shared_timers(&path, marked, class)?;
        let cpp = Self::cpp_timers(&path, marked)?;
        Ok([shared, cpp].concat())
    }

    pub(crate) fn described(timers: &[EventTimer]) -> Value {
        Value::Array(timers.iter().map(Self::entry).collect())
    }

    fn marked(text: &str) -> &str {
        if text.to_ascii_lowercase().contains(Self::MARKER_WORD) {
            return text;
        }
        ""
    }

    fn shared_timers(path: &Path, text: &str, class: &str) -> Result<Vec<EventTimer>, Failure> {
        let (view_path, view_text) = Self::view(path, text);
        let module = crate::events::extract(&view_path, &view_text, Self::REALM)?;
        Ok(Self::owned(module, class))
    }

    fn view(path: &Path, text: &str) -> (PathBuf, String) {
        match Self::extension(path) {
            Self::FSHARP => {
                let view = FsharpView::tac(path, text);
                (view.path().to_path_buf(), view.text().to_owned())
            }
            Self::LUA => {
                let view = LuaView::joined(path, text);
                (view.path().to_path_buf(), view.text().to_owned())
            }
            Self::CPP => (path.to_path_buf(), String::new()),
            _ => (path.to_path_buf(), text.to_owned()),
        }
    }

    fn extension(path: &Path) -> &str {
        path.extension().and_then(OsStr::to_str).unwrap_or_default()
    }

    fn owned(module: Option<EventModule>, class: &str) -> Vec<EventTimer> {
        module
            .into_iter()
            .flat_map(|module| module.types)
            .filter(|event_type| event_type.name == class)
            .flat_map(|event_type| event_type.timers)
            .collect()
    }

    fn cpp_timers(path: &Path, text: &str) -> Result<Vec<EventTimer>, Failure> {
        let wanted = [Self::extension(path) == Self::CPP, !text.is_empty()];
        if wanted != [true, true] {
            return Ok(Vec::new());
        }
        let file = CppFile::read(path, text)?;
        Self::cpp_member_timers(path, &file.class.members)
    }

    fn cpp_member_timers(path: &Path, members: &[CppMember]) -> Result<Vec<EventTimer>, Failure> {
        let timers = members
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
