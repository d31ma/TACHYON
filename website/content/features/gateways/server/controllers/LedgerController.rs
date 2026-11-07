use crate::models::Balance;

#[RequestRoute("/ledger")]
#[Public]
struct LedgerController {
    #[Needs]
    ledger_client: std::sync::Arc<LedgerClient>,
}
impl LedgerController {
    #[GetRoute]
    fn get_ledger(&self) -> Result<Balance, YonResourceError> {
        let balance = self.ledger_client.balance("ACME-7".into()).map_err(|_| YonResourceError)?;
        let forecast = self.ledger_client.forecast(BehaviorValue::Number(1.0)).map_err(|_| YonResourceError)?;
        Ok(Balance { balance: Self::json(&balance)?, forecast: Self::json(&forecast)? })
    }

    fn json(value: &BehaviorValue) -> Result<Json, YonResourceError> {
        let text = value.to_json(1_048_576).map_err(|_| YonResourceError)?;
        Json::parse(&text).map_err(|_| YonResourceError)
    }
}
