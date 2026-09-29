@Entity({"adapter":"publicBilling"})
@Table('invoices')
export default class BillingEntity {
  @Id
  id = 0;
}
