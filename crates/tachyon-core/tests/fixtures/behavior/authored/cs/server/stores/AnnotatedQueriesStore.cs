[Store(typeof(AnnotatedProductEntity), {"fields":["id","name"],"defaultSize":20,"maxSize":100})]
public sealed class AnnotatedQueriesStore {
    [Needs]
    public BehaviorContext behaviors;
    [Query({"operation":"find","where":[{"field":"name","operator":"eq","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
    public async System.Threading.Tasks.Task<BehaviorJSON> findNamed(BehaviorJSON name, [PageDefaults] BehaviorJSON page) { }
    public async System.Threading.Tasks.Task<BehaviorJSON> countByName(BehaviorJSON name) { }
    public async System.Threading.Tasks.Task<BehaviorJSON> existsByName(BehaviorJSON name) { }
}

