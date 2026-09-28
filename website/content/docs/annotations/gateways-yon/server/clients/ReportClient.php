<?php
#[HTTPClient]
class ReportClient {
    #[Gateway({"class":"server/services/ReportService.py"})]
    public function summarize(mixed $period): mixed {}

    #[Gateway({"command":["python3","server/helpers/forecast.py"]})]
    public function forecast(mixed $months): mixed {}
}
