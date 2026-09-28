use super::{PreviewDispatch, PreviewServerOptions, ServerListener, ServerResponse, StaticFiles};
use crate::Failure;
use crate::failure::diagnostic;
use axum::Router;
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct PreviewServer {
    listener: TcpListener,
    application: Router,
    address: SocketAddr,
    root: PathBuf,
}

impl PreviewServer {
    pub async fn bind(
        root: impl AsRef<Path>,
        options: &PreviewServerOptions,
    ) -> Result<Self, Failure> {
        ServerListener::validate_exposure(
            options.host,
            options.allow_non_loopback,
            "preview server",
        )?;
        let root = root.as_ref().to_path_buf();
        Self::require_bundle(&root)?;
        let (listener, address) =
            ServerListener::bind(options.host, options.port, "preview server").await?;
        let state = PreviewDispatch {
            files: StaticFiles::directory(&root, true),
            page_routes: Arc::new(StaticFiles::preview_route_patterns(&root)),
        };
        let page_hosts = crate::safety::DeclaredHosts::published(&root);
        let application = ServerResponse::defensive_headers(
            Router::new().fallback(axum::routing::any(PreviewDispatch::serve).with_state(state)),
            &page_hosts,
        );
        Ok(Self {
            listener,
            application,
            address,
            root,
        })
    }

    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn run_until<F>(self, shutdown: F) -> Result<(), Failure>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        axum::serve(self.listener, self.application)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|error| {
                Failure::one(diagnostic(
                    1303,
                    format!("Preview server stopped unexpectedly: {error}"),
                    Some(String::from("Restart the preview server.")),
                    None,
                ))
            })
    }

    fn require_bundle(root: &Path) -> Result<(), Failure> {
        if std::fs::metadata(root).is_ok_and(|metadata| metadata.is_dir()) {
            return Ok(());
        }
        Err(Failure::one(diagnostic(
            1304,
            format!(
                "Cannot preview missing bundle directory {}.",
                root.display()
            ),
            Some(String::from("Run `ty bundle` before `ty preview`.")),
            None,
        )))
    }
}
