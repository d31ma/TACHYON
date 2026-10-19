pub struct TacPageSocket;

impl TacPageSocket {
    pub fn send(_client: &str, _message: TacValue) -> Result<(), &'static str> {
        Err("HTTP_MISSING")
    }
}
