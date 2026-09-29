@Entity({"adapter":"publicCatalog"})
@Table('products')
export default class ProductEntity {
  /** @type {number} */
  @Id
  id;
  /** @type {string} */
  status;
}
