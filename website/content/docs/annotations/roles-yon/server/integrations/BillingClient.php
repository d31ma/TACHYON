<?php
#[HTTPClient("/billing")]
interface BillingClient {
  #[GetRequest("/surcharge")]
  public function surcharge(): array;
}
