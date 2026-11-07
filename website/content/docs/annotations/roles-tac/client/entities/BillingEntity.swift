@Entity({"adapter":"publicBilling"})
@Table("invoices")
final class BillingEntity {
    @Id
    var id: Int = 0
}
