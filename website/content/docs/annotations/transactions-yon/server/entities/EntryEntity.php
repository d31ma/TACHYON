<?php
#[Entity({"adapter":"ledger"})]
#[Table("entries")]
class EntryEntity {
    #[Id]
    public int $id = 0;
}
