<?php
class ProductReport {
    public function __construct(
        public mixed $active,
        public mixed $count,
        public mixed $others,
    ) {}
}
