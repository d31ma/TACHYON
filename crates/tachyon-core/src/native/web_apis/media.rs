
#[derive(Clone, Copy, Debug, Default)]
pub struct Media;

impl Media {
    const API: &'static str = "media";

    pub fn supported(&self, feature: &str) -> Result<bool, String> {
        Ok(TacWebHost::call(Self::API, "supported", &tac_quote(feature))?.as_flag())
    }

    pub fn devices(&self, request: &str) -> Result<(), String> {
        Self::send("devices", &tac_quote(request))
    }

    pub fn choose_speaker(&self, device: &str) -> Result<(), String> {
        Self::send("chooseSpeaker", &tac_quote(device))
    }

    pub fn capture_microphone(&self, track: &str, device: &str, processing: bool) -> Result<(), String> {
        let arguments = format!("{},{},{processing}", tac_quote(track), tac_quote(device));
        Self::send("captureMicrophone", &arguments)
    }

    pub fn capture_camera(&self, track: &str, device: &str, quality: &str) -> Result<(), String> {
        let arguments = [track, device, quality].map(tac_quote).join(",");
        Self::send("captureCamera", &arguments)
    }

    pub fn capture_screen(&self, track: &str, with_audio: bool) -> Result<(), String> {
        Self::send("captureScreen", &Self::text_and_flag(track, with_audio))
    }

    pub fn stop_track(&self, track: &str) -> Result<(), String> {
        Self::send("stopTrack", &tac_quote(track))
    }

    pub fn mute(&self, track: &str, muted: bool) -> Result<(), String> {
        Self::send("mute", &Self::text_and_flag(track, muted))
    }

    pub fn use_frame_key(&self, key_id: u64, key: &str) -> Result<(), String> {
        let arguments = format!("{key_id},{}", tac_quote(key));
        Self::send("useFrameKey", &arguments)
    }

    pub fn send_frame_key(&self, key_id: u64) -> Result<(), String> {
        Self::send("sendFrameKey", &key_id.to_string())
    }

    pub fn forget_frame_key(&self, key_id: u64) -> Result<(), String> {
        Self::send("forgetFrameKey", &key_id.to_string())
    }

    pub fn encrypt_frames(&self, connection: &str, on: bool) -> Result<(), String> {
        Self::send("encryptFrames", &Self::text_and_flag(connection, on))
    }

    pub fn call_active(&self, on: bool, title: &str) -> Result<(), String> {
        let arguments = format!("{on},{}", tac_quote(title));
        Self::send("callActive", &arguments)
    }

    fn text_and_flag(text: &str, flag: bool) -> String {
        format!("{},{flag}", tac_quote(text))
    }

    fn send(call: &str, arguments: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, call, arguments)?;
        Ok(())
    }
}
