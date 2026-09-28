<?php
#[Entity({"adapter":"catalog"})]
#[Table("products")]
class ProductEntity {
    #[Id]
    public int $id = 0;
}
