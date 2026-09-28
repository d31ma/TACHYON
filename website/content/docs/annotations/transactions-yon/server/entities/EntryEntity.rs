#[Entity({"adapter":"ledger"})]
#[Table("entries")]
struct EntryEntity {
    #[Id]
    id: BehaviorValue,
}
