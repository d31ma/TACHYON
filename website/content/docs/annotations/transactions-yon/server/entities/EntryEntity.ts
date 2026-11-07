@Entity({"adapter":"ledger"})
@Table('entries')
export default class EntryEntity {
  @Id
  id!: number;
  status!: string;
}
