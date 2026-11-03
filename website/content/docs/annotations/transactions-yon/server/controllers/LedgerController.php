<?php
use Models\LedgerReport;

#[RequestRoute("/ledger")]
#[Public]
class LedgerController {
    #[Needs]
    public LedgerService $ledgerService;

    #[GetRoute]
    public function getLedger(): LedgerReport {
        $settled = $this->ledgerService->settle('open');
        return new LedgerReport($settled, LedgerAdapterDelegate::$commands);
    }
}
