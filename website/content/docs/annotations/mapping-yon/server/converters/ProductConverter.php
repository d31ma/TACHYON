<?php
#[Converter]
interface ProductConverter {
    #[Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})]
    public function toSummary(mixed $product): mixed;
}
