<?php
use Models\LedgerReport;

#[RequestRoute("/ledger")]
#[Public]
class LedgerController {
    #[GetRoute]
    public function getLedger(): LedgerReport {
        $context = Behaviors::create(LedgerSetup::class, ['partition' => 'ledger', 'signal' => new BehaviorSignal()]);
        $entries = Behaviors::construct(EntryStore::class, ['behaviors' => $context]);
        $settled = $entries->settle('open');
        $context->dispose();
        return new LedgerReport($settled, LedgerAdapterHelper::$commands);
    }
}
