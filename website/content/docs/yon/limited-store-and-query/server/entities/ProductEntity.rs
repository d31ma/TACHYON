#[Entity({"adapter":"catalog"})]
#[Table("products")]
struct ProductEntity {
    #[Id]
    id: i64,
}
