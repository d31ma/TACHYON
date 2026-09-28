@Service
@Lifetime({"value":"prototype"})
class CartService {
  @Needs
  final CartStore cartStore;
  @Needs
  final CartHelper cartHelper;

  int get total {
    final subtotal = cartStore.subtotal();
    return cartHelper.settle(subtotal);
  }
}
