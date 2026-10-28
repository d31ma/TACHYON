use crate::models::TaxAnswer;

#[RequestRoute("/tax")]
#[Public]
struct TaxController {
    #[Needs]
    tax_delegate: std::sync::Arc<TaxDelegate>,
}
impl TaxController {
    #[GetRoute]
    fn get_tax(&self) -> TaxAnswer {
        TaxAnswer {
            rate: self.tax_delegate.rate("north"),
            total: self.tax_delegate.total(3),
            extra: self.tax_delegate.total_with(3, 4),
        }
    }
}
