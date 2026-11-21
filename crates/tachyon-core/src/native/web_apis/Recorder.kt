class Recorder {
    fun supported(): Boolean = tacBoolean(TacWebHost.call("recorder", "supported"))

    fun start(recording: String, audioTracks: String, videoTrack: String, fileName: String, format: String) {
        TacWebHost.call("recorder", "start", listOf(recording, audioTracks, videoTrack, fileName, format))
    }

    fun addAudio(recording: String, track: String) {
        TacWebHost.call("recorder", "addAudio", listOf(recording, track))
    }

    fun removeAudio(recording: String, track: String) {
        TacWebHost.call("recorder", "removeAudio", listOf(recording, track))
    }

    fun showVideo(recording: String, track: String) {
        TacWebHost.call("recorder", "showVideo", listOf(recording, track))
    }

    fun pause(recording: String) {
        TacWebHost.call("recorder", "pause", listOf(recording))
    }

    fun resume(recording: String) {
        TacWebHost.call("recorder", "resume", listOf(recording))
    }

    fun stop(recording: String) {
        TacWebHost.call("recorder", "stop", listOf(recording))
    }
}
