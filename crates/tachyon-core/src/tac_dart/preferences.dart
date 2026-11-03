final class Preferences {
  const Preferences();

  String scheme() => _text('scheme');

  String motion() => _text('motion');

  String language() => _text('language');

  String _text(String call) => TacWebHost.call('preferences', call) as String;
}
