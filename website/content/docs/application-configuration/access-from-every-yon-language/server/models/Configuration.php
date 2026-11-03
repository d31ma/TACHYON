<?php
class Configuration {
    public function __construct(
        public int $pageSize,
        public int $port,
    ) {}
}
