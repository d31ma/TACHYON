use std::net::{IpAddr, SocketAddr};

#[derive(Clone, Debug, Default)]
pub(super) struct EgressResolver {
    answers: Option<Vec<IpAddr>>,
}

impl EgressResolver {
    pub(super) const fn system() -> Self {
        Self { answers: None }
    }

    #[cfg(test)]
    pub(super) fn fixed(answers: &[IpAddr]) -> Self {
        Self {
            answers: Some(answers.to_vec()),
        }
    }

    pub(super) async fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
        Self::answered_or_looked_up(self.answers.as_deref(), host, port).await
    }

    async fn answered_or_looked_up(
        answers: Option<&[IpAddr]>,
        host: &str,
        port: u16,
    ) -> std::io::Result<Vec<SocketAddr>> {
        if let Some(answers) = answers {
            return Ok(Self::with_port(answers, port));
        }
        Ok(tokio::net::lookup_host((host, port)).await?.collect())
    }

    fn with_port(answers: &[IpAddr], port: u16) -> Vec<SocketAddr> {
        answers
            .iter()
            .map(|address| SocketAddr::new(*address, port))
            .collect()
    }
}
