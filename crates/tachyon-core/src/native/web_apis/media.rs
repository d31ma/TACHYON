
#[derive(Clone, Copy, Debug, Default)]
pub struct Media;

impl Media {
    const API: &'static str = "media";
    const NO_ARGUMENTS: &'static str = "";

    pub fn supported(&self, feature: &str) -> Result<bool, String> {
        Ok(TacWebHost::call(Self::API, "supported", &tac_quote(feature))?.as_flag())
    }

    pub fn devices(&self, request: &str) -> Result<(), String> {
        Self::post("devices", &tac_quote(request))
    }

    pub fn choose_speaker(&self, device: &str) -> Result<(), String> {
        Self::post("chooseSpeaker", &tac_quote(device))
    }

    pub fn capture_microphone(&self, track: &str, device: &str, processing: bool) -> Result<(), String> {
        Self::post("captureMicrophone", &Self::texts_and_flag(&[track, device], processing))
    }

    pub fn capture_camera(&self, track: &str, device: &str, quality: &str) -> Result<(), String> {
        Self::post("captureCamera", &Self::texts(&[track, device, quality]))
    }

    pub fn capture_screen(&self, track: &str, with_audio: bool) -> Result<(), String> {
        Self::post("captureScreen", &Self::texts_and_flag(&[track], with_audio))
    }

    pub fn stop_track(&self, track: &str) -> Result<(), String> {
        Self::post("stopTrack", &tac_quote(track))
    }

    pub fn mute(&self, track: &str, muted: bool) -> Result<(), String> {
        Self::post("mute", &Self::texts_and_flag(&[track], muted))
    }

    pub fn use_frame_key(&self, key_id: u64, key: &str) -> Result<(), String> {
        Self::post("useFrameKey", &[key_id.to_string(), tac_quote(key)].join(","))
    }

    pub fn send_frame_key(&self, key_id: u64) -> Result<(), String> {
        Self::post("sendFrameKey", &key_id.to_string())
    }

    pub fn forget_frame_key(&self, key_id: u64) -> Result<(), String> {
        Self::post("forgetFrameKey", &key_id.to_string())
    }

    pub fn encrypt_frames(&self, connection: &str, on: bool) -> Result<(), String> {
        Self::post("encryptFrames", &Self::texts_and_flag(&[connection], on))
    }

    pub fn call_active(&self, on: bool, title: &str) -> Result<(), String> {
        Self::post("callActive", &format!("{on},{}", tac_quote(title)))
    }

    pub fn send(&self, connection: &str, track: &str, stream: &str, simulcast: bool) -> Result<(), String> {
        Self::post("send", &Self::texts_and_flag(&[connection, track, stream], simulcast))
    }

    pub fn replace(&self, connection: &str, track: &str, with_track: &str) -> Result<(), String> {
        Self::post("replace", &Self::texts(&[connection, track, with_track]))
    }

    pub fn unsend(&self, connection: &str, track: &str) -> Result<(), String> {
        Self::post("unsend", &Self::texts(&[connection, track]))
    }

    pub fn stats(&self, request: &str, connection: &str) -> Result<(), String> {
        Self::post("stats", &Self::texts(&[request, connection]))
    }

    pub fn speaking(&self, on: bool) -> Result<(), String> {
        Self::post("speaking", &on.to_string())
    }

    pub fn resume_audio(&self) -> Result<(), String> {
        Self::post("resumeAudio", Self::NO_ARGUMENTS)
    }

    fn texts(values: &[&str]) -> String {
        values.iter().map(|value| tac_quote(value)).collect::<Vec<_>>().join(",")
    }

    fn texts_and_flag(values: &[&str], flag: bool) -> String {
        format!("{},{flag}", Self::texts(values))
    }

    fn post(call: &str, arguments: &str) -> Result<(), String> {
        TacWebHost::call(Self::API, call, arguments)?;
        Ok(())
    }
}
