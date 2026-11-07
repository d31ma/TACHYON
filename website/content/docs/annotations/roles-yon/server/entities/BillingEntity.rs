#[Entity({"adapter":"billing"})]
#[Table("invoices")]
struct BillingEntity {
    #[Id]
    id: i64,
}
