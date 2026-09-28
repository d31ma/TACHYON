final class Connectivity {
  const Connectivity();

  bool online() => TacWebHost.call('connectivity', 'online') as bool;
}
