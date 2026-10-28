import 'models/TaxAnswer.dart';

@RequestRoute("/tax")
@Public
class TaxController {
  @Needs
  final TaxDelegate taxDelegate;

  static const int _count = 3;

  @GetRoute
  TaxAnswer getTax() {
    return TaxAnswer(
      taxDelegate.rate("north"),
      taxDelegate.total(_count),
      taxDelegate.totalWith(_count, 4),
    );
  }
}
