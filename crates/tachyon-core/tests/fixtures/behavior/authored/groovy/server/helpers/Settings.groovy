@Settings({"prefix":"YON_APP_"})
class Settings {
    String title
    boolean enabled = false
    @Min(1)
    @Max(100)
    int count = 2
    double ratio = 1.5
}

