import 'models/CatalogSummary.dart';

@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  final ProductConverter converter;

  @GetRoute
  CatalogSummary getCatalog() {
    final summary = converter.toSummary({
      'detail': {'id': 7},
      'name': 'Anvil',
      'extra': 'dropped',
    });
    return CatalogSummary(summary);
  }
}
