#[tac::entity({"adapter":"publicBilling"})]
#[tac::table("invoices")]
struct BillingEntity {
    #[Id]
    id: i64,
}
