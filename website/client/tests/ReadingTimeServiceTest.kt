@Test(SiteTestSetup::class, {"partition":"tests"})
class ReadingTimeServiceTest {
    fun testAShortPageTakesOneMinute() {
        Assert.equals("About 1 minute to read", ReadingTimeService().label(0))
        Assert.equals("About 1 minute to read", ReadingTimeService().label(200))
    }

    fun testAPartMinuteCountsAsAWholeMinute() {
        Assert.equals("About 2 minutes to read", ReadingTimeService().label(201))
        Assert.equals("About 6 minutes to read", ReadingTimeService().label(1150))
    }
}
