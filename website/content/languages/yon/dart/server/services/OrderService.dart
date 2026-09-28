@Service
class OrderService {
  static List<String> list() => OrderStore.all();
}
