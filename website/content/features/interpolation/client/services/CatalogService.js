@Service
export default class CatalogService {
  title = 'Catalog'
  count = 6
  /** @type {{ name: string, price: string }} */
  product = { name: 'Anvil', price: '£42.00' }
}