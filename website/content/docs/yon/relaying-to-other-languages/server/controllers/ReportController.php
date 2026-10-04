<?php
#[RequestRoute("/report")]
#[Public]
class ReportController {
    #[GetRoute]
    public function getReport(): void {
        Yon::relay(['perl', 'server/relays/report.pl']);
    }
}
