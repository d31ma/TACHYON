[Store(typeof(EntryEntity), {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})]
public interface EntryStore {
    System.Threading.Tasks.Task<BehaviorJSON> findByStatus(BehaviorJSON status);
    System.Threading.Tasks.Task<BehaviorJSON> countByStatus(BehaviorJSON status);
}
