[Store(typeof(BillingEntity), {"fields":["id"],"defaultSize":20,"maxSize":100})]
public interface BillingStore {
  System.Threading.Tasks.Task<BehaviorJSON> existsById(BehaviorJSON id);
}
