@Service
class LedgerService {
  late BehaviorContext behaviors;
  @Needs
  final EntryStore entries;

  @OnStart
  void open() {
    behaviors = Behaviors.create(LedgerSetup, partition: 'ledger', signal: BehaviorSignal());
  }

  @AllOrNothing({"adapter":"ledger"})
  Future<Object?> settle(String status) async {
    final open = await entries.findByStatus(status);
    final total = await entries.countByStatus(status);
    return {'open': open, 'total': total};
  }
}
