<?php
use Models\Item;
use Models\ItemList;

#[RequestRoute("/items")]
#[Public]
class ItemsController {
    #[GetRoute]
    public function getItems(): ItemList {
        return new ItemList([]);
    }

    #[PostRoute]
    #[ResponseStatus(201)]
    public function postItem(#[FromBody] Item $item): Item {
        return $item;
    }
}
