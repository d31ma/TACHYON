@RequestRoute("/catalog")
@Public
export default class CatalogController {
  @Needs
  readonly catalogClient!: CatalogClient;

  @GetRoute
  async getCatalog() {
    return {
      found: await this.catalogClient.find("anvil", "full", "acme"),
      saved: await this.catalogClient.save({ sku: "anvil", level: 3 }),
      replaced: await this.catalogClient.replace(5, { sku: "anvil", level: 4 }),
      renamed: await this.catalogClient.rename(6, { sku: "hammer", level: 1 }),
      removed: await this.catalogClient.remove(7),
    };
  }
}
