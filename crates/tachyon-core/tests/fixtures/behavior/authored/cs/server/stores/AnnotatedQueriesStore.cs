[Store(typeof(AnnotatedProductEntity), {"fields":["id","name"],"defaultSize":20,"maxSize":100,"setup":"AnnotatedConfigSetup"})]
public interface AnnotatedQueriesStore {
    [Query({"operation":"find","where":[{"field":"name","operator":"eq","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
    System.Threading.Tasks.Task<BehaviorJSON> findNamed(BehaviorJSON name, [PageDefaults] BehaviorJSON page);
    System.Threading.Tasks.Task<BehaviorJSON> countByName(BehaviorJSON name);
    System.Threading.Tasks.Task<BehaviorJSON> existsByName(BehaviorJSON name);
}
