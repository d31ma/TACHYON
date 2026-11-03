@Entity({"adapter":"ledger"})
@Table("entries")
class EntryEntity {
    @Id
    var id: Long = 0
}
