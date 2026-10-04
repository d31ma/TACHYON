<?php

#[Settings({"prefix":"YON_CATALOG_"})]
class CatalogSettings {
    #[Min(1)]
    #[Max(100)]
    public int $pageSize = 20;
}
