<?php
use Models\Balance;

#[RequestRoute("/ledger")]
#[Public]
class LedgerController {
    #[Needs]
    public LedgerClient $ledgerClient;

    #[GetRoute]
    public function getLedger(): Balance {
        return new Balance($this->ledgerClient->balance('ACME-7'), $this->ledgerClient->forecast(1));
    }
}
