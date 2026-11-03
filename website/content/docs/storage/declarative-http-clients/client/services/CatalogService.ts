@Service
export default class CatalogService {
  @Needs
  readonly catalogClient!: CatalogClient;
  products: BehaviorJsonValue[] = []
  error = ''

  async load() {
    await this.catalogClient.all().then((products) => this.show(products), () => this.showFailure())
  }

  private show(products: unknown): void {
    if (!Array.isArray(products)) return this.showFailure()
    this.products = products
    this.error = ''
  }

  private showFailure(): void {
    this.error = 'Products could not be loaded. Please retry.'
  }
}