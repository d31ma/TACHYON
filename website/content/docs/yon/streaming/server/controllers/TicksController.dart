import 'models/Tick.dart';

@RequestRoute("/ticks")
@Public
class TicksController {
  @StreamRoute
  Stream<Tick> streamTicks() async* {
    yield* _ticksUpTo(3);
  }

  Stream<Tick> _ticksUpTo(int last) async* {
    for (var level = 1; level <= last; level++) {
      yield Tick('ANVIL-1', level);
    }
  }
}
