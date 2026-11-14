class Media {
    fun supported(feature: String): Boolean = tacBoolean(TacWebHost.call("media", "supported", listOf(feature)))

    fun devices(request: String) {
        TacWebHost.call("media", "devices", listOf(request))
    }

    fun chooseSpeaker(device: String) {
        TacWebHost.call("media", "chooseSpeaker", listOf(device))
    }

    fun captureMicrophone(track: String, device: String, processing: Boolean) {
        TacWebHost.call("media", "captureMicrophone", listOf(track, device, processing))
    }

    fun captureCamera(track: String, device: String, quality: String) {
        TacWebHost.call("media", "captureCamera", listOf(track, device, quality))
    }

    fun captureScreen(track: String, withAudio: Boolean) {
        TacWebHost.call("media", "captureScreen", listOf(track, withAudio))
    }

    fun stopTrack(track: String) {
        TacWebHost.call("media", "stopTrack", listOf(track))
    }

    fun mute(track: String, muted: Boolean) {
        TacWebHost.call("media", "mute", listOf(track, muted))
    }
}
