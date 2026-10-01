-- @Test(SiteTestSetup, {"partition":"tests"})
---@class ReadingTimeServiceTest
ReadingTimeServiceTest = {}

function ReadingTimeServiceTest:test_a_short_page_takes_one_minute()
  local readingTime = self:construct(ReadingTimeService)
  Assert.equals("About 1 minute to read", readingTime:label(0))
  Assert.equals("About 1 minute to read", readingTime:label(200))
end

function ReadingTimeServiceTest:test_a_part_minute_counts_as_a_whole_minute()
  local readingTime = self:construct(ReadingTimeService)
  Assert.equals("About 2 minutes to read", readingTime:label(201))
  Assert.equals("About 6 minutes to read", readingTime:label(1150))
end
