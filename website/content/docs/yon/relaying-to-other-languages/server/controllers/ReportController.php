<?php
#[RequestRoute("/report")]
#[Public]
class ReportController {
    #[GetRoute]
    public function getReport(): void {
        Yon::relay(['ruby', 'server/helpers/ReportDelegate.rb']);
    }
}
