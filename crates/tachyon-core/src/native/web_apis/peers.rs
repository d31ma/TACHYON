
#[derive(Clone, Copy, Debug, Default)]
pub struct Peers;

impl Peers {
    const API: &'static str = "peers";
    const NO_ARGUMENTS: &'static str = "";

    pub fn supported(&self) -> Result<bool, String> {
        Ok(TacWebHost::call(Self::API, "supported", Self::NO_ARGUMENTS)?.as_flag())
    }

    pub fn open(&self, username: &str, credential: &str) -> Result<String, String> {
        let arguments = [username, credential].map(tac_quote).join(",");
        Ok(TacWebHost::call(Self::API, "open", &arguments)?.as_text())
    }

    pub fn offer(&self, connection: &str) -> Result<(), String> {
        Self::send_call("offer", &[connection])
    }

    pub fn answer(&self, connection: &str, offer_sdp: &str) -> Result<(), String> {
        Self::send_call("answer", &[connection, offer_sdp])
    }

    pub fn accept(&self, connection: &str, answer_sdp: &str) -> Result<(), String> {
        Self::send_call("accept", &[connection, answer_sdp])
    }

    pub fn add_candidate(&self, connection: &str, candidate_json: &str) -> Result<(), String> {
        Self::send_call("addCandidate", &[connection, candidate_json])
    }

    pub fn restart(&self, connection: &str) -> Result<(), String> {
        Self::send_call("restart", &[connection])
    }

    pub fn open_channel(&self, connection: &str, label: &str, ordered: bool) -> Result<String, String> {
        let arguments = format!("{},{},{ordered}", tac_quote(connection), tac_quote(label));
        Ok(TacWebHost::call(Self::API, "openChannel", &arguments)?.as_text())
    }

    pub fn send(&self, connection: &str, channel: &str, data: &str) -> Result<bool, String> {
        let arguments = [connection, channel, data].map(tac_quote).join(",");
        Ok(TacWebHost::call(Self::API, "send", &arguments)?.as_flag())
    }

    pub fn close_channel(&self, connection: &str, channel: &str) -> Result<(), String> {
        Self::send_call("closeChannel", &[connection, channel])
    }

    pub fn close(&self, connection: &str) -> Result<(), String> {
        Self::send_call("close", &[connection])
    }

    fn send_call(call: &str, values: &[&str]) -> Result<(), String> {
        let arguments = values.iter().map(|value| tac_quote(value)).collect::<Vec<_>>().join(",");
        TacWebHost::call(Self::API, call, &arguments)?;
        Ok(())
    }
}
