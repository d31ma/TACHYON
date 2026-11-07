final class WindowControls {
  const WindowControls();

  bool granted(String control) => TacWebHost.call('windowControls', 'granted', [control]) as bool;

  void minimize() => TacWebHost.call('windowControls', 'minimize');

  void maximize() => TacWebHost.call('windowControls', 'maximize');

  void fullscreen() => TacWebHost.call('windowControls', 'fullscreen');

  void close() => TacWebHost.call('windowControls', 'close');

  void resize(int width, int height) => TacWebHost.call('windowControls', 'resize', [width, height]);

  void setTitle(String text) => TacWebHost.call('windowControls', 'setTitle', [text]);
}
