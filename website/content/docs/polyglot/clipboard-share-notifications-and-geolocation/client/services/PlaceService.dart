@Service
class PlaceService {
  @Needs()
  final Geolocation geolocation;
  @Needs()
  final Clipboard clipboard;
  String place = '';
  String problem = '';

  void locate() => _keepProblem(() => geolocation.currentPosition());

  void copy() => _keepProblem(() => clipboard.writeText(place));

  @OnEvent
  void found(PositionFound found) {
    place = '${found.latitude}, ${found.longitude}';
  }

  @OnEvent
  void failed(BrowserCallFailed failed) {
    problem = failed.message;
  }

  void _keepProblem(void Function() call) {
    try {
      call();
    } on StateError catch (refusal) {
      problem = refusal.message;
    }
  }
}
