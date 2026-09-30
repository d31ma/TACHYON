#[HTTPClient]
class LedgerClient
  # @param account [String]
  # @return [void]
  #[Gateway({"class":"server/services/LedgerService.py"})]
  def balance(account)
  end

  # @param months [Integer]
  # @return [void]
  #[Gateway({"command":["python3","server/helpers/forecast.py"]})]
  def forecast(months)
  end
end
