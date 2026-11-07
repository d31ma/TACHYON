use crate::Failure;
use crate::failure::diagnostic;

pub(super) struct ProcessFailure;

impl ProcessFailure {
    pub(super) fn timed_out() -> Failure {
        Failure::one(diagnostic(
            2110,
            "Handler invocation exceeded its deadline.",
            Some(String::from(
                "Reduce handler work or choose a larger bounded timeout.",
            )),
            None,
        ))
    }

    pub(super) fn cancelled() -> Failure {
        Failure::one(diagnostic(
            2111,
            "Handler invocation was cancelled.",
            Some(String::from(
                "Retry only if the caller still needs the result.",
            )),
            None,
        ))
    }

    pub(super) fn stopped_accepting_work() -> Failure {
        Failure::one(diagnostic(
            2101,
            "Handler supervisor stopped accepting work.",
            Some(String::from("Create a new supervisor and retry.")),
            None,
        ))
    }

    pub(super) fn missing_pipe(name: &str) -> Failure {
        Failure::one(diagnostic(
            2101,
            format!("Handler runtime did not provide a piped {name} stream."),
            None,
            None,
        ))
    }

    pub(super) fn gateways_need_process_backend() -> Failure {
        Failure::one(diagnostic(
            2010,
            "Gateways require the Yon process backend.",
            Some(String::from(
                "The Firecracker transport does not yet transfer invocation-scoped resource capabilities.",
            )),
            None,
        ))
    }

    pub(super) fn stream(message: &str) -> Failure {
        Failure::one(diagnostic(
            2107,
            format!("Streamed handler failed: {message}"),
            Some(String::from(
                "Fix the handler stream or its protocol adapter, then retry the request.",
            )),
            None,
        ))
    }
}
