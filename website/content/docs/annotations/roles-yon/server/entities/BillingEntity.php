<?php
#[Entity({"adapter":"billing"})]
#[Table("invoices")]
class BillingEntity {
    #[Id]
    public int $id = 0;
}
