use super::{ServerResponse, StaticFiles};
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use std::sync::Arc;
use tower_http::services::ServeDir;

#[derive(Clone, Debug)]
pub(in crate::server) struct PreviewDispatch {
    pub(in crate::server) files: ServeDir,
    pub(in crate::server) page_routes: Arc<Vec<String>>,
}

impl PreviewDispatch {
    pub(in crate::server) async fn serve(
        State(state): State<Self>,
        request: Request<Body>,
    ) -> Response<Body> {
        StaticFiles::serve(&state.files, &state.page_routes, request)
            .await
            .unwrap_or_else(|| {
                ServerResponse::text(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cannot serve the preview bundle.",
                )
            })
    }
}
