[Store(typeof(ProductEntity), {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})]
public interface ProductStore {
    System.Threading.Tasks.Task<BehaviorJSON> findByStatus(BehaviorJSON status);
    System.Threading.Tasks.Task<BehaviorJSON> countByStatus(BehaviorJSON status);
    [Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
    System.Threading.Tasks.Task<BehaviorJSON> excluding(BehaviorJSON status, [PageDefaults] BehaviorJSON page);
}
