@Entity({"adapter":"catalog"})
@Table('products')
export default class ProductEntity {
  /** @type {number} */
  @Id
  id;
  /** @type {string} */
  status;
}
