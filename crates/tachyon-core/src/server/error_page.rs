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
    const STYLE: &str = ":root { color-scheme: light dark; --text: #1f2937; --muted: #4b5563; \
         --surface: #ffffff; --panel: #f3f4f6; --accent: #b91c1c; }\n\
         @media (prefers-color-scheme: dark) { :root { --text: #e5e7eb; --muted: #9ca3af; \
         --surface: #111827; --panel: #1f2937; --accent: #f87171; } }\n\
         body { margin: 0; background: var(--surface); color: var(--text); \
         font: 16px/1.6 ui-sans-serif, system-ui, sans-serif; }\n\
         main { box-sizing: border-box; max-width: 56rem; margin: 0 auto; \
         padding: max(2rem, env(safe-area-inset-top)) max(1rem, env(safe-area-inset-right)) \
         max(2rem, env(safe-area-inset-bottom)) max(1rem, env(safe-area-inset-left)); }\n\
         h1 { margin: 0 0 0.5rem; font-size: clamp(1.5rem, 4vw, 2rem); line-height: 1.25; }\n\
         p { margin: 0 0 1.5rem; color: var(--muted); }\n\
         pre { margin: 0; padding: 1rem 1.25rem; border-left: 4px solid var(--accent); \
         border-radius: 0.5rem; background: var(--panel); white-space: pre-wrap; \
         overflow-wrap: anywhere; font: 14px/1.6 ui-monospace, SFMono-Regular, Menlo, monospace; }\n";

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
             <title>{title}</title>\n<style>\n{style}</style>\n</head>\n<body data-tachyon-environment=\"development\" data-tachyon-error-page>\n\
             <main>\n<h1>{title}</h1>\n\
             <p>Fix the problems below. The preview keeps watching and opens the site as soon as the project builds.</p>\n\
             <pre>{problems}</pre>\n</main>\n{script}\n</body>\n</html>\n",
            title = Self::TITLE,
            style = Self::STYLE,
            problems = PageHead::escape(&problems),
            script = self.hot_updates.script_tag(),
        )
    }
}
