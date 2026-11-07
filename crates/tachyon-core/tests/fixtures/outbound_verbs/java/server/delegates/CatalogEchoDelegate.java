@Delegate
class CatalogEchoDelegate {
    static Object echo(java.util.Map<String, Object> request, Behaviors.Execution execution) {
        java.util.Map<String, Object> sent = new java.util.LinkedHashMap<>();
        for (String key : java.util.List.of("method", "path", "query", "headers", "body")) sent.put(key, request.get(key));
        return sent;
    }
}
