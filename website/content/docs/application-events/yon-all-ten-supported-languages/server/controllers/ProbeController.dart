import 'models/ProbeReady.dart';

@RequestRoute("/probe")
@Public
class ProbeController {
  @Needs
  final ProbeService probe;
  @Needs
  final PublishService publish;

  @GetRoute
  ProbeReady? getProbe() {
    return probe.last;
  }
}
