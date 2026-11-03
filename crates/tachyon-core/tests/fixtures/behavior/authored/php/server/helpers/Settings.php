<?php
#[Settings({"prefix":"YON_CATALOG_"})]
class Settings {
    #[Min(1)]
    #[Max(100)]
    public int $size = 20;
    public bool $enabled = false;
}

