<?php
use Models\Item;

#[RequestRoute("/items")]
#[Public]
class ItemsController {
    #[GetRoute("/{id}")]
    public function getItem(#[FromPath] string $id): Item {
        return new Item($id);
    }

    #[PostRoute]
    #[ResponseStatus(201)]
    public function postItem(#[FromBody] Item $item): Item {
        return $item;
    }
}
