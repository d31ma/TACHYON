@Entity({"adapter":"billing"})
@Table("invoices")
class BillingEntity {
    @Id
    var id: Long = 0
}
