public sealed class Media
{
    private const string Api = "media";

    public bool Supported(string feature) => TacWebHost.Call(Api, "supported", new JsonArray(feature)).GetValue<bool>();

    public void Devices(string request) => TacWebHost.Call(Api, "devices", new JsonArray(request));

    public void ChooseSpeaker(string device) => TacWebHost.Call(Api, "chooseSpeaker", new JsonArray(device));

    public void CaptureMicrophone(string track, string device, bool processing) => TacWebHost.Call(Api, "captureMicrophone", new JsonArray(track, device, processing));

    public void CaptureCamera(string track, string device, string quality) => TacWebHost.Call(Api, "captureCamera", new JsonArray(track, device, quality));

    public void CaptureScreen(string track, bool withAudio) => TacWebHost.Call(Api, "captureScreen", new JsonArray(track, withAudio));

    public void StopTrack(string track) => TacWebHost.Call(Api, "stopTrack", new JsonArray(track));

    public void Mute(string track, bool muted) => TacWebHost.Call(Api, "mute", new JsonArray(track, muted));

    public void UseFrameKey(long keyId, string key) => TacWebHost.Call(Api, "useFrameKey", new JsonArray(keyId, key));

    public void SendFrameKey(long keyId) => TacWebHost.Call(Api, "sendFrameKey", new JsonArray(keyId));

    public void ForgetFrameKey(long keyId) => TacWebHost.Call(Api, "forgetFrameKey", new JsonArray(keyId));

    public void EncryptFrames(string connection, bool on) => TacWebHost.Call(Api, "encryptFrames", new JsonArray(connection, on));

    public void CallActive(bool on, string title) => TacWebHost.Call(Api, "callActive", new JsonArray(on, title));

    public void Send(string connection, string track, string stream, bool simulcast) => TacWebHost.Call(Api, "send", new JsonArray(connection, track, stream, simulcast));

    public void Replace(string connection, string track, string withTrack) => TacWebHost.Call(Api, "replace", new JsonArray(connection, track, withTrack));

    public void Unsend(string connection, string track) => TacWebHost.Call(Api, "unsend", new JsonArray(connection, track));

    public void Stats(string request, string connection) => TacWebHost.Call(Api, "stats", new JsonArray(request, connection));

    public void Speaking(bool on) => TacWebHost.Call(Api, "speaking", new JsonArray(on));

    public void ResumeAudio() => TacWebHost.Call(Api, "resumeAudio", new JsonArray());
}
