@Service
class ProductService {
    @Needs
    ProductStore productStore;
    String summary = "Not loaded.";

    void load() {
        java.util.List<?> rows = (java.util.List<?>) productStore.excluding("retired", java.util.Map.of("size", 5));
        Object activeCount = productStore.countByStatus("active");
        summary = rows.size() + " of " + activeCount + " active";
    }
}
