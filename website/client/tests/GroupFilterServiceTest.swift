@Test(SiteTestSetup, {"partition":"tests"})
final class GroupFilterServiceTest {
  func testNoChoiceShowsEveryGroup() async throws {
    if !GroupFilterService().shows("Routing") { throw BehaviorError("NO_CHOICE_SHOWS_ALL") }
  }

  func testPickShowsOnlyTheChosenGroup() async throws {
    let filter = GroupFilterService()
    filter.pick("Native")
    if !filter.shows("Native") { throw BehaviorError("CHOSEN_GROUP_SHOWS") }
    if filter.shows("Routing") { throw BehaviorError("OTHER_GROUPS_HIDE") }
    filter.pick("")
    if !filter.shows("Routing") { throw BehaviorError("ALL_SHOWS_EVERY_GROUP_AGAIN") }
  }
}
