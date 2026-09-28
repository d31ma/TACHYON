import 'models/CatalogSummary.dart';

@RequestRoute("/catalog")
@Public
class CatalogController {
  @GetRoute
  CatalogSummary getCatalog() {
    final context = Behaviors.create(CatalogSetup, partition: 'public-catalog', signal: BehaviorSignal());
    final mapper = ProductConverter(context);
    final summary = mapper.toSummary({
      'detail': {'id': 7},
      'name': 'Anvil',
      'extra': 'dropped',
    });
    context.dispose();
    return CatalogSummary(summary);
  }
}
