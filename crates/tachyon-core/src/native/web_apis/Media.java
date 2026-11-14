final class Media {
    private static final String API = "media";

    boolean supported(String feature) {
        return Boolean.TRUE.equals(TacWebHost.call(API, "supported", feature));
    }

    void devices(String request) {
        TacWebHost.call(API, "devices", request);
    }

    void chooseSpeaker(String device) {
        TacWebHost.call(API, "chooseSpeaker", device);
    }

    void captureMicrophone(String track, String device, boolean processing) {
        TacWebHost.call(API, "captureMicrophone", track, device, processing);
    }

    void captureCamera(String track, String device, String quality) {
        TacWebHost.call(API, "captureCamera", track, device, quality);
    }

    void captureScreen(String track, boolean withAudio) {
        TacWebHost.call(API, "captureScreen", track, withAudio);
    }

    void stopTrack(String track) {
        TacWebHost.call(API, "stopTrack", track);
    }

    void mute(String track, boolean muted) {
        TacWebHost.call(API, "mute", track, muted);
    }

    void useFrameKey(long keyId, String key) {
        TacWebHost.call(API, "useFrameKey", keyId, key);
    }

    void sendFrameKey(long keyId) {
        TacWebHost.call(API, "sendFrameKey", keyId);
    }

    void forgetFrameKey(long keyId) {
        TacWebHost.call(API, "forgetFrameKey", keyId);
    }

    void encryptFrames(String connection, boolean on) {
        TacWebHost.call(API, "encryptFrames", connection, on);
    }
}
