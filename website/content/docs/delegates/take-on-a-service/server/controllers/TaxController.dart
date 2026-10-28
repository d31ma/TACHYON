import 'models/TaxAnswer.dart';

@RequestRoute("/tax")
@Public
class TaxController {
  @Needs
  final TaxDelegate taxDelegate;

  @GetRoute
  TaxAnswer getTax() {
    return TaxAnswer(
      taxDelegate.rate("north"),
      taxDelegate.total(3),
      taxDelegate.totalWith(3, 4),
    );
  }
}
