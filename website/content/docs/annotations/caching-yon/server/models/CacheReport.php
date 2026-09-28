<?php
class CacheReport {
    public function __construct(
        public mixed $first,
        public mixed $repeat,
        public mixed $written,
        public mixed $refreshed,
    ) {}
}
