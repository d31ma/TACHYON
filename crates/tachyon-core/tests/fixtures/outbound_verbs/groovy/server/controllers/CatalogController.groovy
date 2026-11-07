@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    final CatalogClient catalog

    @GetRoute
    Object getCatalog() {
        java.util.Map<String, Object> answer = new java.util.LinkedHashMap<>()
        answer.put("found", catalog.find("anvil", "full", "acme"))
        answer.put("saved", catalog.save(java.util.Map.of("sku", "anvil", "level", 3)))
        answer.put("replaced", catalog.replace(5L, java.util.Map.of("sku", "anvil", "level", 4)))
        answer.put("renamed", catalog.rename(6L, java.util.Map.of("sku", "hammer", "level", 1)))
        answer.put("removed", catalog.remove(7L))
        return answer
    }
}
