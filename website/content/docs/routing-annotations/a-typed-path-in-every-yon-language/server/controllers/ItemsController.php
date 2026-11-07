<?php
use Models\Item;

#[RequestRoute("/items")]
#[Public]
class ItemsController {
    #[GetRoute("/{id}")]
    public function getItem(#[FromPath] int $id): Item {
        return new Item($id);
    }
}
