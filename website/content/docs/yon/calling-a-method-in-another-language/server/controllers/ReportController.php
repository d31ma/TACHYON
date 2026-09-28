<?php
use Models\Report;

#[RequestRoute("/report")]
#[Public]
class ReportController {
    #[Needs]
    public ReportClient $reportClient;

    #[GetRoute]
    public function getReport(): Report {
        return new Report($this->reportClient->summarize('2026-Q3'), $this->reportClient->forecast(2));
    }
}
