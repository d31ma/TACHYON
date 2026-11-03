@Settings({"prefix":"YON_APP_"})
class Settings {
    var title: String? = null
    var enabled: Boolean = false
    @Min(1)
    @Max(100)
    var count: Int = 2
    var ratio: Double = 1.5
}

