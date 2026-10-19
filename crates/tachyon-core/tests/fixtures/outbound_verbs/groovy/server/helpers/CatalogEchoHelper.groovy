@Helper
class CatalogEchoHelper {
    static Object echo(java.util.Map<String, Object> request) {
        java.util.Map<String, Object> sent = new java.util.LinkedHashMap<>()
        for (String key : ["method", "path", "query", "headers", "body"]) sent.put(key, request.get(key))
        return sent
    }
}
