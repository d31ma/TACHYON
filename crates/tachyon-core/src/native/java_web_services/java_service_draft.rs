use super::java_member::JavaMember;
use super::java_service::JavaService;
use crate::events::EventModule;

pub(super) struct JavaServiceDraft {
    pub(super) service: JavaService,
    pub(super) required: Vec<String>,
    pub(super) bases: Vec<String>,
    pub(super) events: Option<EventModule>,
    pub(super) held: Vec<JavaMember>,
    pub(super) behavior_source: String,
}
