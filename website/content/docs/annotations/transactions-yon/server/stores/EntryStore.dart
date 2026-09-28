@Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class EntryStore {
  @Needs
  final BehaviorContext behaviors;

  Future<Object?> findByStatus(String status) async {}

  Future<Object?> countByStatus(String status) async {}

  @AllOrNothing({"adapter":"ledger"})
  Future<Object?> settle(String status) async {
    final open = await findByStatus(status);
    final total = await countByStatus(status);
    return {'open': open, 'total': total};
  }
}
