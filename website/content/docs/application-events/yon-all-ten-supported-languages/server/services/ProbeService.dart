import 'models/ProbeReady.dart';

@Service
class ProbeService {
  ProbeReady? last;

  @OnEvent
  void observe(ProbeReady ready) {
    last = ready;
  }

  @OnStop
  void destroy() {
    last = null;
  }
}
