@Setup
class ApplicationSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()
    }
}
