@Entity({"adapter":"publicBilling"})
@Table("invoices")
class BillingEntity {
    @Id
    var id: Long = 0
}
