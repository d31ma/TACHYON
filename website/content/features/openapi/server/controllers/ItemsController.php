<?php
use Models\Item;

#[RequestRoute("/items")]
#[Public]
class ItemsController {
    #[GetRoute("/{id}")]
    public function getItem(#[FromPath] int $id): Item {
        return new Item('anvil', $id);
    }

    #[PatchRoute("/{id}")]
    public function patchItem(#[FromPath] int $id, #[FromBody] Item $item): Item {
        return new Item($item->name, $id);
    }
}
