<?php
class LedgerReport {
    public function __construct(
        public mixed $settled,
        public mixed $commands,
    ) {}
}
