final class Focus {
  const Focus();

  void move(String target) => TacWebHost.call('focus', 'move', [target]);

  void trap(String container) => TacWebHost.call('focus', 'trap', [container]);

  void release() => TacWebHost.call('focus', 'release');
}
