[Test(typeof(SiteTestSetup), {"partition":"tests"})]
sealed class LanguagesServiceTest {
    void testHandlersFlattenEveryEntrysFiles() {
        var entry = (int files) => (object)new Dictionary<string, object> { ["id"] = "csharp", ["files"] = Enumerable.Repeat((object)"file", files).ToArray() };
        var languages = new LanguagesService { Entries = new object[] { entry(2), entry(3) } };
        Assert.equals(5, ((List<object>)languages.Handlers()).Count, "handlers flattens every entry's files");
        Assert.equals(0, ((List<object>)new LanguagesService { Entries = null }.Handlers()).Count, "no entries is no handlers");
    }
}
