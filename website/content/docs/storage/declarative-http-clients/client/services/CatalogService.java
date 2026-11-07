import java.util.ArrayList;
import java.util.List;

@Service
class CatalogService {
    @Needs
    private final CatalogClient catalog;
    List<String> products = new ArrayList<>();
    String error = "";

    void load() {
        show(fetch());
    }

    private Object fetch() {
        try {
            return catalog.all();
        } catch (RuntimeException failure) {
            return null;
        }
    }

    private void show(Object answer) {
        if (!(answer instanceof List<?> items)) {
            showFailure();
            return;
        }
        products = items.stream().map(String::valueOf).toList();
        error = "";
    }

    private void showFailure() {
        error = "Products could not be loaded. Please retry.";
    }
}
