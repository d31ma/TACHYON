<?php
#[Entity({"adapter":"data"})]
#[Table("products")]
class ProductEntity {
    #[Id]
    public int $id = 0;
}

