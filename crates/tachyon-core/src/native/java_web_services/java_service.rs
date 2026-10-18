use super::java_constructor_provider::JavaConstructorProvider;
use super::java_member::JavaMember;
use crate::events::EventModule;
use std::path::PathBuf;

pub(super) struct JavaService {
    pub(super) instance: String,
    pub(super) class: String,
    pub(super) members: Vec<JavaMember>,
    pub(super) required: Vec<String>,
    pub(super) providers: Vec<JavaConstructorProvider>,
    pub(super) path: PathBuf,
    pub(super) source: String,
}

pub(super) struct JavaServiceDraft {
    pub(super) service: JavaService,
    pub(super) required: Vec<String>,
    pub(super) events: Option<EventModule>,
    pub(super) held: Vec<JavaMember>,
    pub(super) behavior_source: String,
}
