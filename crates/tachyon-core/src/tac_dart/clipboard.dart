final class Clipboard {
  const Clipboard();

  static const _api = 'clipboard';

  void writeText(String text) => TacWebHost.call(_api, 'writeText', [text]);

  void readText() => TacWebHost.call(_api, 'readText');
}
