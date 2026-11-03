@Entity({"adapter":"billing"})
@Table('invoices')
export default class BillingEntity {
  @Id
  id = 0;
}
