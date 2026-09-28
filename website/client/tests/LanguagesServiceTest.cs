[Test(typeof(SiteTestSetup), {"partition":"tests"})]
sealed class LanguagesServiceTest {
    void testHandlersFlattenEveryEntrysFiles() {
        var entry = (int files) => (object)new Dictionary<string, object> { ["id"] = "csharp", ["files"] = Enumerable.Repeat((object)"file", files).ToArray() };
        var languages = new LanguagesService { Entries = new object[] { entry(2), entry(3) } };
        if (((List<object>)languages.Handlers()).Count != 5) throw new System.Exception("handlers flattens every entry's files");
        if (((List<object>)new LanguagesService { Entries = null }.Handlers()).Count != 0) throw new System.Exception("no entries is no handlers");
    }
}
