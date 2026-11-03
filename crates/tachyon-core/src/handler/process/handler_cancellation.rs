use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Default)]
pub struct HandlerCancellation {
    token: CancellationToken,
}

impl HandlerCancellation {
    pub fn cancel(&self) {
        self.token.cancel();
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub(in crate::handler) async fn cancelled(&self) {
        self.token.cancelled().await;
    }

    pub(in crate::handler) fn child(&self) -> Self {
        Self {
            token: self.token.child_token(),
        }
    }
}
