@Test(SiteTestSetup, {"partition":"tests"})
final class FeatureFilterServiceTest {
  func testAnEmptyQueryShowsEveryFeature() async throws {
    if !FeatureFilterService().shows("Page routing", "Folders define the pages.") { throw BehaviorError("EMPTY_QUERY_SHOWS_ALL") }
  }

  func testEveryWordMustMatchTheTitleOrTheSummary() async throws {
    let filter = FeatureFilterService()
    filter.find("  Folders ROUTING ")
    if !filter.shows("Page routing", "Folders define the pages.") { throw BehaviorError("WORDS_MATCH_ANY_CASE") }
    if filter.shows("Browser storage", "Folders define the pages.") { throw BehaviorError("EVERY_WORD_MUST_MATCH") }
  }
}
