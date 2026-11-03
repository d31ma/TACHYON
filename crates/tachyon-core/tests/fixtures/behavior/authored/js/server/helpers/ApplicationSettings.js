@Settings({"prefix":"TAC_TEST_"})
class ApplicationSettings {
    /** @type {string} */
    label;
    enabled = false;
    @Min(1)
    @Max(100)
    limit = 20;
    @Min(0)
    @Max(10)
    ratio = 1.5;
}

