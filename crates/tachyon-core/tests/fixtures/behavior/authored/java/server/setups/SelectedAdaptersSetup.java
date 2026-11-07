@Setup
@WhenProfile("test", "development")
@WhenSetting({"name":"YON_ENABLED","havingValue":"true","matchIfMissing":false})
class SelectedAdaptersSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) { return RuntimeAdaptersSetup.configure(execution); }
}

