final class Clock {
  const Clock();

  double nowMs() => (TacWebHost.call('clock', 'now') as num).toDouble();

  double monotonicMs() =>
      (TacWebHost.call('clock', 'monotonic') as num).toDouble();

  String timeZone() => TacWebHost.call('clock', 'timeZone') as String;
}
