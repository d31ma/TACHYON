<?php
class ValidationService {
    public function accept(#[Check(ProductInput::class)] array $value): array {
        return $value;
    }
}

