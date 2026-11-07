import 'models/Tick.dart';

@RequestRoute("/ticks")
@Public
class TicksController {
  @StreamRoute
  Stream<Tick> streamTicks() async* {
    yield* _countdown(3);
  }

  Stream<Tick> _countdown(int start) async* {
    for (var level = start; level >= 1; level--) {
      yield Tick('ANVIL-1', level);
    }
  }
}
