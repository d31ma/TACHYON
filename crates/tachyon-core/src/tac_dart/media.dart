final class Media {
  const Media();

  static const _api = 'media';

  bool supported(String feature) =>
      TacWebHost.call(_api, 'supported', [feature]) as bool;

  void devices(String request) => TacWebHost.call(_api, 'devices', [request]);

  void chooseSpeaker(String device) =>
      TacWebHost.call(_api, 'chooseSpeaker', [device]);

  void captureMicrophone(String track, String device, bool processing) =>
      TacWebHost.call(_api, 'captureMicrophone', [track, device, processing]);

  void captureCamera(String track, String device, String quality) =>
      TacWebHost.call(_api, 'captureCamera', [track, device, quality]);

  void captureScreen(String track, bool withAudio) =>
      TacWebHost.call(_api, 'captureScreen', [track, withAudio]);

  void stopTrack(String track) => TacWebHost.call(_api, 'stopTrack', [track]);

  void mute(String track, bool muted) =>
      TacWebHost.call(_api, 'mute', [track, muted]);
}
