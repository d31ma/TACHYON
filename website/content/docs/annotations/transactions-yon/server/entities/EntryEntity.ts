@Entity({"adapter":"ledger"})
@Table('entries')
class EntryEntity {
  @Id
  id!: number;
}
