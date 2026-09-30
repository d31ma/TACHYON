<?php
#[HTTPClient]
interface LedgerClient {
    #[Gateway({"class":"server/services/LedgerService.py"})]
    public function balance(mixed $account): mixed;

    #[Gateway({"command":["python3","server/helpers/forecast.py"]})]
    public function forecast(mixed $months): mixed;
}
