#[derive(Clone, Copy, Debug, Default)]
pub struct Recorder;

impl Recorder {
    const API: &'static str = "recorder";
    const NO_ARGUMENTS: &'static str = "";

    pub fn supported(&self) -> Result<bool, String> {
        Ok(TacWebHost::call(Self::API, "supported", Self::NO_ARGUMENTS)?.as_flag())
    }

    pub fn start(&self, recording: &str, audio_tracks: &str, video_track: &str, file_name: &str, format: &str) -> Result<(), String> {
        Self::tell("start", recording, &[audio_tracks, video_track, file_name, format])
    }

    pub fn add_audio(&self, recording: &str, track: &str) -> Result<(), String> {
        Self::tell("addAudio", recording, &[track])
    }

    pub fn remove_audio(&self, recording: &str, track: &str) -> Result<(), String> {
        Self::tell("removeAudio", recording, &[track])
    }

    pub fn show_video(&self, recording: &str, track: &str) -> Result<(), String> {
        Self::tell("showVideo", recording, &[track])
    }

    pub fn pause(&self, recording: &str) -> Result<(), String> {
        Self::tell("pause", recording, &[])
    }

    pub fn resume(&self, recording: &str) -> Result<(), String> {
        Self::tell("resume", recording, &[])
    }

    pub fn stop(&self, recording: &str) -> Result<(), String> {
        Self::tell("stop", recording, &[])
    }

    fn tell(call: &str, recording: &str, more: &[&str]) -> Result<(), String> {
        let mut values = vec![tac_quote(recording)];
        values.extend(more.iter().map(|value| tac_quote(value)));
        TacWebHost::call(Self::API, call, &values.join(","))?;
        Ok(())
    }
}
