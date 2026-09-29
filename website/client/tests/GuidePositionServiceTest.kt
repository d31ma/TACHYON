@Test(SiteTestSetup::class, {"partition":"tests"})
class GuidePositionServiceTest {
    private val order: List<String> = listOf("install", "first-app", "services")

    fun testAGuideNamesItsPlaceInTheReadingOrder() {
        Assert.equals("Guide 1 of 3", GuidePositionService().label(order, "install"))
        Assert.equals("Guide 3 of 3", GuidePositionService().label(order, "services"))
    }

    fun testAPageOutsideTheOrderHasNoPlace() {
        Assert.equals("", GuidePositionService().label(order, "nowhere"))
    }
}
