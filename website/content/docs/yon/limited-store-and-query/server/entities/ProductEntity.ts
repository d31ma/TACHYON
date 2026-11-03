@Entity({"adapter":"catalog"})
@Table('products')
export default class ProductEntity {
  @Id
  id!: number;
  status!: string;
}
