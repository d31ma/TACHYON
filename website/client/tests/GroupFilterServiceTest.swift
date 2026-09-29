@Test(SiteTestSetup, {"partition":"tests"})
final class GroupFilterServiceTest {
  func testNoChoiceShowsEveryGroup() async throws {
    try Assert.isTrue(GroupFilterService().shows("Routing"), "no choice shows every group")
  }

  func testPickShowsOnlyTheChosenGroup() async throws {
    let filter = GroupFilterService()
    filter.pick("Native")
    try Assert.isTrue(filter.shows("Native"), "the chosen group shows")
    try Assert.equals(false, filter.shows("Routing"), "the other groups hide")
    filter.pick("")
    try Assert.isTrue(filter.shows("Routing"), "no choice shows every group again")
  }
}
