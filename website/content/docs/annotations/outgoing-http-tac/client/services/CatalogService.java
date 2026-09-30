@Service
class CatalogService {
    private final CatalogClient catalog = new CatalogClient();
    int count = 0;

    void load() {
        java.util.Map<?, ?> answer = (java.util.Map<?, ?>) catalog.all();
        count = ((java.util.List<?>) answer.get("items")).size();
    }
}
