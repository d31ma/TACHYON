<?php

#[Service]
class ProductService {
    public function accept(#[Check(ProductInput::class)] array $input): array {
        return $input;
    }
}
