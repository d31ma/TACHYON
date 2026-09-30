import 'models/Price.dart';

@RequestRoute("/price")
@Public
class PriceController {
  @Needs
  final PriceService priceService;

  @GetRoute
  Price getPrice() => priceService.priceFor(20, 3);
}
