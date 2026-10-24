use super::{DiagnosticEntry, HostGuard, HotUpdateHub, ProducerTasks, ServerResponse};
use crate::compiler::PageHead;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, Response, StatusCode};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(in crate::server) struct ErrorPage {
    pub(in crate::server) hot_updates: HotUpdateHub,
    pub(in crate::server) producers: ProducerTasks,
    pub(in crate::server) stopping: CancellationToken,
}

impl ErrorPage {
    const TITLE: &str = "Tachyon cannot start this project";

    pub(in crate::server) fn into_router(self, guard: HostGuard) -> Router {
        let routes =
            Router::new().fallback(
                axum::routing::any(
                    |State(page): State<Self>, request: Request<Body>| async move {
                        page.answer(&request)
                    },
                )
                .with_state(self),
            );
        ServerResponse::defensive_headers(
            guard.protect(routes),
            &crate::safety::DeclaredHosts::default(),
        )
    }

    fn answer(&self, request: &Request<Body>) -> Response<Body> {
        match request.uri().path() {
            HotUpdateHub::HOT_ENDPOINT => {
                self.hot_updates
                    .stream(request, &self.producers, &self.stopping)
            }
            HotUpdateHub::LIVE_SCRIPT_ENDPOINT => ServerResponse::live_script(),
            HotUpdateHub::DIAGNOSTICS_ENDPOINT => self.hot_updates.diagnostics_response(),
            _ => self.document(),
        }
    }

    fn document(&self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(self.html()))
            .unwrap_or_else(|_| ServerResponse::text(StatusCode::SERVICE_UNAVAILABLE, Self::TITLE))
    }

    fn html(&self) -> String {
        let problems = DiagnosticEntry::text(&self.hot_updates.current_diagnostics());
        format!(
            "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <title>{title}</title>\n</head>\n<body data-tachyon-environment=\"development\" data-tachyon-error-page>\n\
             <main>\n<h1>{title}</h1>\n\
             <p>Fix the problems below. The preview keeps watching and opens the site as soon as the project builds.</p>\n\
             <pre>{problems}</pre>\n</main>\n{script}\n</body>\n</html>\n",
            title = Self::TITLE,
            problems = PageHead::escape(&problems),
            script = self.hot_updates.script_tag(),
        )
    }
}
