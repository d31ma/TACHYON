@Service
class CatalogService {
    @Needs
    private final CatalogClient catalog;
    int count = 0;

    void load() {
        java.util.Map<?, ?> answer = (java.util.Map<?, ?>) catalog.all();
        count = ((java.util.List<?>) answer.get("items")).size();
    }
}
