@Test(SiteTestSetup, {"partition": "tests"})
class LanguagesServiceTest:
    def test_handlers_flatten_every_entrys_files(self) -> None:
        languages = LanguagesService()
        languages.entries = [{"id": "csharp", "files": ["file", "file"]},
                             {"id": "csharp", "files": ["file", "file", "file"]}]
        Assert.equals(5, len(languages.handlers()), "handlers flattens every entry's files")
        languages.entries = None
        Assert.equals(0, len(languages.handlers()), "no entries is no handlers")

    def test_handlers_preserve_file_order_and_contents(self) -> None:
        first = {"name": "server/controllers/FirstController.py", "code": "first"}
        last = {"name": "server/controllers/LastController.java", "code": "last"}
        languages = LanguagesService()
        languages.entries = [{"files": [first]}, {"files": []}, {"files": [last]}]
        Assert.equals([first, last], languages.handlers(), "file order and structured contents stay unchanged")

    def test_handlers_ignore_malformed_entries_and_non_array_files(self) -> None:
        languages = LanguagesService()
        languages.entries = [None, "entry", {}, {"files": None}, {"files": "not an array"},
                             {"files": ["kept"]}]
        Assert.equals(["kept"], languages.handlers(), "malformed entries do not discard real files")
        languages.entries = "not an array"
        Assert.equals([], languages.handlers(), "a non-array catalogue is empty")
