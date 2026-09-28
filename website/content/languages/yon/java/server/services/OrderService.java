@Service
class OrderService {
    static java.util.List<String> list() {
        return OrderStore.all();
    }
}
