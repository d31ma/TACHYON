final class Recorder {
  const Recorder();

  static const _api = 'recorder';

  bool supported() => TacWebHost.call(_api, 'supported') as bool;

  void start(
    String recording,
    String audioTracks,
    String videoTrack,
    String fileName,
    String format,
  ) => TacWebHost.call(_api, 'start', [
    recording,
    audioTracks,
    videoTrack,
    fileName,
    format,
  ]);

  void addAudio(String recording, String track) =>
      TacWebHost.call(_api, 'addAudio', [recording, track]);

  void removeAudio(String recording, String track) =>
      TacWebHost.call(_api, 'removeAudio', [recording, track]);

  void showVideo(String recording, String track) =>
      TacWebHost.call(_api, 'showVideo', [recording, track]);

  void pause(String recording) => TacWebHost.call(_api, 'pause', [recording]);

  void resume(String recording) => TacWebHost.call(_api, 'resume', [recording]);

  void stop(String recording) => TacWebHost.call(_api, 'stop', [recording]);
}
