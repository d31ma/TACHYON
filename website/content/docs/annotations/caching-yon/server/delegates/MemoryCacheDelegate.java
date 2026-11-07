@Delegate
class MemoryCacheDelegate implements Behaviors.CacheAdapter {
    final java.util.Map<String, Behaviors.Entry> entries = new java.util.HashMap<>();

    public Behaviors.Entry get(String key, Behaviors.Execution execution) {
        return entries.getOrDefault(key, new Behaviors.Entry());
    }

    public void set(String key, Object value, Behaviors.Write write) {
        entries.put(key, new Behaviors.Entry(value, write.expiresAt));
    }

    public void delete(String key, Behaviors.Execution execution) {
        entries.remove(key);
    }
}
