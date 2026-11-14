final class CodeScanner {
  const CodeScanner();

  static const _api = 'codeScanner';

  bool supported() => TacWebHost.call(_api, 'supported') as bool;

  void scan() => TacWebHost.call(_api, 'scan');

  void cancel() => TacWebHost.call(_api, 'cancel');
}
