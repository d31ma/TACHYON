final class SecureRandom {
  const SecureRandom();

  List<int> bytes(int count) =>
      (TacWebHost.call('secureRandom', 'bytes', [count]) as List<Object?>)
          .cast<int>();

  String uuid() => TacWebHost.call('secureRandom', 'uuid') as String;
}
