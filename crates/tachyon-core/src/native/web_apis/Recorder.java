final class Recorder {
    private static final String API = "recorder";

    boolean supported() {
        return TacWebHost.flag(API, "supported");
    }

    void start(String recording, String audioTracks, String videoTrack, String fileName, String format) {
        TacWebHost.call(API, "start", recording, audioTracks, videoTrack, fileName, format);
    }

    void addAudio(String recording, String track) {
        TacWebHost.call(API, "addAudio", recording, track);
    }

    void removeAudio(String recording, String track) {
        TacWebHost.call(API, "removeAudio", recording, track);
    }

    void showVideo(String recording, String track) {
        TacWebHost.call(API, "showVideo", recording, track);
    }

    void pause(String recording) {
        TacWebHost.call(API, "pause", recording);
    }

    void resume(String recording) {
        TacWebHost.call(API, "resume", recording);
    }

    void stop(String recording) {
        TacWebHost.call(API, "stop", recording);
    }
}
