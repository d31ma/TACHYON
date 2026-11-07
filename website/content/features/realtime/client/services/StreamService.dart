@Service
class StreamService {
  static const _tickLimit = 128;

  @Needs
  final TicksClient ticksClient;
  List<Tick> ticks = [];

  @OnEvent
  void addTick(Tick tick) {
    ticks = _latest([...ticks, tick]);
  }

  List<Tick> _latest(List<Tick> all) =>
      all.length > _tickLimit ? all.sublist(all.length - _tickLimit) : all;
}
