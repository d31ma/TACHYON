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
}
