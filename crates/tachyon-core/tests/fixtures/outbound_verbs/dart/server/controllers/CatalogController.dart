@RequestRoute("/catalog")
@Public
class CatalogController {
  @Needs
  final CatalogClient catalog;

  @GetRoute
  Future<Object?> getCatalog() async {
    return {
      'found': await catalog.find("anvil", "full", "acme"),
      'saved': await catalog.save({'sku': "anvil", 'level': 3}),
      'replaced': await catalog.replace(5, {'sku': "anvil", 'level': 4}),
      'renamed': await catalog.rename(6, {'sku': "hammer", 'level': 1}),
      'removed': await catalog.remove(7),
    };
  }
}
