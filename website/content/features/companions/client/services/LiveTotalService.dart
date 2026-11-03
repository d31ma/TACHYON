import 'models/CartTotal.dart';

@Service
class LiveTotalService {
  int seen = 0;

  @OnEvent({"replay":true})
  void changed(CartTotal changed) { seen = changed.total; }
}
