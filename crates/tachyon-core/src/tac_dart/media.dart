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

  void useFrameKey(int keyId, String key) =>
      TacWebHost.call(_api, 'useFrameKey', [keyId, key]);

  void sendFrameKey(int keyId) =>
      TacWebHost.call(_api, 'sendFrameKey', [keyId]);

  void forgetFrameKey(int keyId) =>
      TacWebHost.call(_api, 'forgetFrameKey', [keyId]);

  void encryptFrames(String connection, bool on) =>
      TacWebHost.call(_api, 'encryptFrames', [connection, on]);

  void callActive(bool on, String title) =>
      TacWebHost.call(_api, 'callActive', [on, title]);
}
