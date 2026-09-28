<?php
#[HTTPClient]
class ReportClient {
    #[Gateway({"class":"server/services/ReportService.py"})]
    public function summarize(string $period): void {}

    #[Gateway({"command":["python3","server/helpers/forecast.py"]})]
    public function forecast(int $months): void {}
}
