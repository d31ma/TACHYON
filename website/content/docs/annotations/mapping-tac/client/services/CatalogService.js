@Service
export default class CatalogService {
  title = 'Not mapped.';
  async load() {
    const options = { partition: 'public-catalog', signal: AbortSignal.timeout(5000) };
    const context = await Behaviors.create(CatalogSetup, options);
    const mapper = Behaviors.construct(ProductConverter, { behaviors: context });
    const product = { detail: { id: 7 }, name: 'Anvil', extra: 'dropped' };
    const summary = await mapper.toSummary(product);
    this.title = `${summary.kind} ${summary.productId}: ${summary.title}`;
    context.dispose();
  }
}
