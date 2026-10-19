#[cfg(not(target_arch = "wasm32"))]
pub struct TacNativeHttpRequest {
    pub method: &'static str,
    pub origin: &'static str,
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub deadline: u64,
    pub limit: usize,
}

#[cfg(not(target_arch = "wasm32"))]
impl TacNativeHttpRequest {
    const SECURE: &'static str = "https://";
    const PLAIN: &'static str = "http://";
    const SECURE_PORT: u16 = 443;
    const PLAIN_PORT: u16 = 80;

    pub fn secure(&self) -> bool {
        self.origin.starts_with(Self::SECURE)
    }

    pub fn authority(&self) -> &'static str {
        let origin = self.origin;
        origin
            .strip_prefix(Self::SECURE)
            .or_else(|| origin.strip_prefix(Self::PLAIN))
            .unwrap_or(origin)
    }

    pub fn host(&self) -> &'static str {
        let authority = self.authority();
        let bracketed = authority.strip_prefix('[').and_then(|rest| rest.split(']').next());
        bracketed.unwrap_or_else(|| authority.rsplit_once(':').map_or(authority, |(host, _)| host))
    }

    pub fn port(&self) -> u16 {
        let authority = self.authority();
        let written = authority
            .rsplit_once(':')
            .filter(|(_, port)| !port.contains(']'))
            .and_then(|(_, port)| port.parse().ok());
        let fallback = [Self::PLAIN_PORT, Self::SECURE_PORT][usize::from(self.secure())];
        written.unwrap_or(fallback)
    }

    pub fn remaining_ms(&self) -> u64 {
        self.deadline.saturating_sub(TacNativeHttp::now_ms())
    }
}
