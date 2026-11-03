final class Geolocation {
  const Geolocation();

  void currentPosition() => TacWebHost.call('geolocation', 'currentPosition');
}
