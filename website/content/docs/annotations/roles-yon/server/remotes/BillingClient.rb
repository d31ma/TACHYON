#[HTTPClient("/billing")]
class BillingClient
  # @return [void]
  #[GetRequest("/surcharge")]
  def surcharge
  end
end
