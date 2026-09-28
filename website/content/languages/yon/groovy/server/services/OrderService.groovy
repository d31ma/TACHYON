@Service
class OrderService {
    static List<String> list() {
        return OrderStore.all()
    }
}
