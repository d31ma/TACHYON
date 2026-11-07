@Service
class OrderService {
  @Needs
  final OrdersClient ordersClient;

  Future<SentRequest> replace() => ordersClient.replace('a-7', {'item': 'hammer'}, 'north');

  Future<SentRequest> change() => ordersClient.change('a-7', {'item': 'nails'}, 'south');

  Future<SentRequest> remove() => ordersClient.remove('a-7', 'duplicate');
}
