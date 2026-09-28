import 'models/Message.dart';

@RequestRoute("/echo")
@Public
class EchoController {
  @PostRoute
  Message postEcho(@FromBody Message message) => message;
}
