[Store(typeof(ProductEntity), {"fields":["id","status"],"defaultSize":20,"maxSize":100})]
public sealed class ProductStore {
    [Needs]
    public BehaviorContext behaviors;
    public async System.Threading.Tasks.Task<BehaviorJSON> findByStatus(BehaviorJSON status) {}
    public async System.Threading.Tasks.Task<BehaviorJSON> countByStatus(BehaviorJSON status) {}
    [Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
    public async System.Threading.Tasks.Task<BehaviorJSON> excluding(BehaviorJSON status, [PageDefaults] BehaviorJSON page) {}
    public BehaviorJSON bounded([PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] BehaviorJSON request) { return request; }
}
