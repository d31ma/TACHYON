@Entity({"adapter":"publicCatalog"})
@Table("products")
class ProductEntity {
    @Id
    long id = 0;
    String status = "";
}
