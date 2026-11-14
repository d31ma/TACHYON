class Media:
    API = 'media'

    def supported(self, feature: str) -> bool:
        return _TacPythonWebHost.call(self.API, 'supported', (feature,)) is True

    def devices(self, request: str) -> None:
        _TacPythonWebHost.call(self.API, 'devices', (request,))

    def choose_speaker(self, device: str) -> None:
        _TacPythonWebHost.call(self.API, 'chooseSpeaker', (device,))

    def capture_microphone(self, track: str, device: str, processing: bool) -> None:
        _TacPythonWebHost.call(self.API, 'captureMicrophone', (track, device, processing))

    def capture_camera(self, track: str, device: str, quality: str) -> None:
        _TacPythonWebHost.call(self.API, 'captureCamera', (track, device, quality))

    def capture_screen(self, track: str, with_audio: bool) -> None:
        _TacPythonWebHost.call(self.API, 'captureScreen', (track, with_audio))

    def stop_track(self, track: str) -> None:
        _TacPythonWebHost.call(self.API, 'stopTrack', (track,))

    def mute(self, track: str, muted: bool) -> None:
        _TacPythonWebHost.call(self.API, 'mute', (track, muted))
