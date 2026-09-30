[<HTTPClient>]
type LedgerClient =
    [<Gateway({"class":"server/services/LedgerService.py","method":"balance"})>]
    abstract Balance: account: BehaviorJSON -> BehaviorJSON

    [<Gateway({"command":["python3","server/helpers/forecast.py"]})>]
    abstract Forecast: months: BehaviorJSON -> BehaviorJSON
