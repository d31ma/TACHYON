import java.util.List;
import models.Tab;

@Service
class BasketService {
    int count = 0;
    String label = "";
    private String selected = "";
    List<Tab> tabs = List.of(new Tab("a", "A"));

    void increment() {
        count += 1;
    }

    void select(String id) {
        selected = id;
    }
}
