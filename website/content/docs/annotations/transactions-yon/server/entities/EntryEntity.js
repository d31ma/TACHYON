@Entity({"adapter":"ledger"})
@Table('entries')
class EntryEntity {
  /** @type {number} */
  @Id
  id;
}
