public sealed class Recorder
{
    private const string Api = "recorder";

    public bool Supported() => TacWebHost.Call(Api, "supported", new JsonArray()).GetValue<bool>();

    public void Start(string recording, string audioTracks, string videoTrack, string fileName, string format) => TacWebHost.Call(Api, "start", new JsonArray(recording, audioTracks, videoTrack, fileName, format));

    public void AddAudio(string recording, string track) => TacWebHost.Call(Api, "addAudio", new JsonArray(recording, track));

    public void RemoveAudio(string recording, string track) => TacWebHost.Call(Api, "removeAudio", new JsonArray(recording, track));

    public void ShowVideo(string recording, string track) => TacWebHost.Call(Api, "showVideo", new JsonArray(recording, track));

    public void Pause(string recording) => TacWebHost.Call(Api, "pause", new JsonArray(recording));

    public void Resume(string recording) => TacWebHost.Call(Api, "resume", new JsonArray(recording));

    public void Stop(string recording) => TacWebHost.Call(Api, "stop", new JsonArray(recording));
}
