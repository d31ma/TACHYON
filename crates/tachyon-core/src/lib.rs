mod annotation_layout;
mod annotation_modules;
mod annotation_rules;
mod application;
mod behavior;
pub mod cache;
pub mod client_tests;
mod companion;
mod compiler;
mod composed_types;
mod convention;
mod declared_contract;
pub mod doctor;
mod events;
mod external_command;
mod failure;
mod fsharp_source;
#[cfg(feature = "fuzzing")]
pub mod fuzzing;
mod go_source;
mod groovy_home;
mod groovy_source;
mod handler;
mod hot_update;
mod html;
mod lexical;
mod lua_source;
pub mod method_coverage;
mod migrate;
mod model_layout;
mod model_sources;
mod native;
mod project;
mod python_parameters;
mod route_types;
mod routing;
mod ruby_source;
mod safety;
mod scaffold;
mod schedule;
mod server;
mod storage;
mod tac_dart;
mod tac_lua;
mod tac_python;
mod tac_ruby;
mod tac_web;
mod template;
mod tool_restart;
pub mod tool_setup;
mod topics;
mod ttid;
mod wasm_optimizer;
mod wasm_release_note;
mod wasm_release_step;
mod web_api_clients;
mod web_runtime;
mod yon;

pub use annotation_layout::SimplerForms;
pub use application::ApplicationConfig;
pub use compiler::{
    ApiBaselineCheck, BuildOptions, BuildProfile, BuildResult, CompressionTotals, WebCompiler,
};
pub use composed_types::ComposedTypes;
pub use failure::Failure;
pub use handler::{
    EnvironmentPolicy, FirecrackerIsolation, HandlerCancellation, HandlerLanguage,
    HandlerRuntimePrograms, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
    TopicDeliveries, Workers, YonIsolationPolicy,
};
pub use html::{HtmlDocument, HtmlFrontend};
pub use migrate::{
    MigrationAnalysis, MigrationFinding, MigrationFix, MigrationFixPlan, MigrationReport,
    MigrationStatus,
};
pub use native::{
    NativeBuildOptions, NativeBuildResult, NativeCompiler, NativeUiChange, NativeUiContract,
    NativeUiGeneration, native_target_directory,
};
pub use project::{
    CompanionKind, HandlerNode, NativeCompanion, Project, ProjectDiscovery, RouteGraph, RouteNode,
    ViewKind,
};
pub use routing::{RouteMatch, match_route};
pub use safety::LocalHostName;
pub use scaffold::{
    AdditionResult, AppFeature, AppTarget, Choice, ChoiceAnswers, ChoiceList, ChoicePrompt,
    ProjectAddition, ProjectChoices, RealmChoice, Scaffold, ScaffoldResult, TacLanguage,
    YonLanguage,
};
pub use server::{
    DevServer, DevServerOptions, ErrorPageServer, ErrorPageWait, PreviewServer,
    PreviewServerOptions,
};
pub use tool_restart::ToolRestart;
pub use ttid::{created_at_milliseconds as ttid_created_at, generate as generate_ttid};

pub(crate) fn without_bom(source: &str) -> &str {
    source.strip_prefix('\u{feff}').unwrap_or(source)
}
