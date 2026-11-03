@Entity({"adapter":"ledger"})
@Table('entries')
export default class EntryEntity {
  /** @type {number} */
  @Id
  id;
  /** @type {string} */
  status;
}
