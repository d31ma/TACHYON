class Recorder:
    API = 'recorder'

    def supported(self) -> bool:
        return _TacPythonWebHost.call(self.API, 'supported') is True

    def start(self, recording: str, audio_tracks: str, video_track: str, file_name: str, format: str) -> None:
        _TacPythonWebHost.call(self.API, 'start', (recording, audio_tracks, video_track, file_name, format))

    def add_audio(self, recording: str, track: str) -> None:
        _TacPythonWebHost.call(self.API, 'addAudio', (recording, track))

    def remove_audio(self, recording: str, track: str) -> None:
        _TacPythonWebHost.call(self.API, 'removeAudio', (recording, track))

    def show_video(self, recording: str, track: str) -> None:
        _TacPythonWebHost.call(self.API, 'showVideo', (recording, track))

    def pause(self, recording: str) -> None:
        _TacPythonWebHost.call(self.API, 'pause', (recording,))

    def resume(self, recording: str) -> None:
        _TacPythonWebHost.call(self.API, 'resume', (recording,))

    def stop(self, recording: str) -> None:
        _TacPythonWebHost.call(self.API, 'stop', (recording,))
