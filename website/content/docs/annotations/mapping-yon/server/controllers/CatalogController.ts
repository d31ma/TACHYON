import type { CatalogSummary } from '/models/CatalogSummary.ts'

@RequestRoute("/catalog")
@Public
class CatalogController {
  @GetRoute
  async getCatalog(): Promise<CatalogSummary> {
    const options = { partition: 'public-catalog', signal: AbortSignal.timeout(5000) };
    const context = await Behaviors.create(CatalogSetup, options);
    const mapper = Behaviors.construct(ProductConverter, { behaviors: context });
    const product = { detail: { id: 7 }, name: 'Anvil', extra: 'dropped' };
    const summary = await mapper.toSummary(product);
    context.dispose();
    return { summary };
  }
}
