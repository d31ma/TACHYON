<?php
class OrderReport {
    public function __construct(
        public mixed $submitted,
        public mixed $report,
    ) {}
}
