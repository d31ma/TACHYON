class Clock {
    fun nowMs(): Double = tacDouble(TacWebHost.call("clock", "now"))

    fun monotonicMs(): Double = tacDouble(TacWebHost.call("clock", "monotonic"))

    fun timeZone(): String = tacString(TacWebHost.call("clock", "timeZone"))
}
