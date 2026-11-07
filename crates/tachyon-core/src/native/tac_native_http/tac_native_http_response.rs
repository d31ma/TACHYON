#[cfg(not(target_arch = "wasm32"))]
pub struct TacNativeHttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}
