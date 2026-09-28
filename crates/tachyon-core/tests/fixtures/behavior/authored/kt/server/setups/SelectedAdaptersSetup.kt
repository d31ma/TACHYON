@Setup
@WhenProfile("test", "development")
@WhenSetting({"name":"YON_ENABLED","havingValue":"true","matchIfMissing":false})
class SelectedAdaptersSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration { return RuntimeAdaptersSetup.configure(execution) }
    }
}

