@Test(SiteTestSetup, {"partition":"tests"})
final class FeatureFilterServiceTest {
  func testAnEmptyQueryShowsEveryFeature() async throws {
    try Assert.isTrue(FeatureFilterService().shows("Page routing", "Folders define the pages."), "an empty query shows every feature")
  }

  func testEveryWordMustMatchTheTitleOrTheSummary() async throws {
    let filter = FeatureFilterService()
    filter.find("  Folders ROUTING ")
    try Assert.isTrue(filter.shows("Page routing", "Folders define the pages."), "words match in any case")
    try Assert.equals(false, filter.shows("Browser storage", "Folders define the pages."), "every word must match")
  }
}
