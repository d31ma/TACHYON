#[Entity({"adapter":"publicCatalog"})]
#[Table("products")]
struct ProductEntity {
    #[Id]
    id: i64,
}
