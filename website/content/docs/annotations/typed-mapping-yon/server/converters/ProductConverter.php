<?php
use Models\Product;
use Models\Summary;
#[Converter]
interface ProductConverter {
#[Convert({"fields":[{"target":"kind","constant":"product"}]})]
public function map(Product $product): Summary;
}
