[HTTPClient]
public interface LedgerClient {
    [Gateway({"class":"server/services/LedgerService.py","method":"balance"})]
    BehaviorJSON Balance(BehaviorJSON account);

    [Gateway({"command":["python3","server/helpers/forecast.py"]})]
    BehaviorJSON Forecast(BehaviorJSON months);
}
