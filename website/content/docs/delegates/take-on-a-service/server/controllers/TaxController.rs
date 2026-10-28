use crate::models::TaxAnswer;

#[RequestRoute("/tax")]
#[Public]
struct TaxController {
    #[Needs]
    tax_delegate: std::sync::Arc<TaxDelegate>,
}
impl TaxController {
    const COUNT: i64 = 3;

    #[GetRoute]
    fn get_tax(&self) -> TaxAnswer {
        TaxAnswer {
            rate: self.tax_delegate.rate("north"),
            total: self.tax_delegate.total(Self::COUNT),
            extra: self.tax_delegate.total_with(Self::COUNT, 4),
        }
    }
}
