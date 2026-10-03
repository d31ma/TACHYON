@Test(SiteTestSetup, {"partition":"tests"})
export default class LanguagesServiceTest {
  async testHandlersListEveryEntrysFilesInOrder() {
    const first = { name: 'server/controllers/FirstController.py', code: 'first' }
    const middle = { name: 'server/controllers/MiddleController.rb', code: 'middle' }
    const last = { name: 'server/controllers/LastController.java', code: 'last' }
    const languages = new LanguagesService()
    languages.entries = [
      { id: 'python', name: 'Python', files: [first, middle] },
      { id: 'go', name: 'Go', files: [] },
      { id: 'java', name: 'Java', files: [last] },
    ]
    Assert.equals('first,middle,last', languages.handlers().map(file => file.code).join(','), 'every file, in catalogue order')
  }

  async testNoEntriesIsNoHandlers() {
    const languages = new LanguagesService()
    languages.entries = []
    Assert.equals(0, languages.handlers().length)
  }
}
