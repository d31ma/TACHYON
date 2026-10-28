[Delegate]
sealed class CatalogTransportDelegate : BehaviorHTTPAdapter {
    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"items\":[\"anvil\"]}"));
    }
}
