data class Pulse(val amount: Int)

data class Relay(val amount: Int)

data class Cycle(val amount: Int)

data class Isolation(val value: Int)

data class Ordered(val status: String, val total: Int)

data class Other(val status: String, val total: Int)

data class Echo(val amount: Int)
