@HTTPClient
export default class LedgerClient {
  @Gateway({"class":"server/services/LedgerService.py"})
  async balance(account: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}

  @Gateway({"command":["python3","server/helpers/forecast.py"]})
  async forecast(months: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}
}
