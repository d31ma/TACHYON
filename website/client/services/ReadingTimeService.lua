-- @Service
---@class ReadingTimeService
ReadingTimeService = {}
ReadingTimeService.WORDS_PER_MINUTE = 200
ReadingTimeService.ONE_MINUTE = 1
ReadingTimeService.SINGULAR = "About 1 minute to read"
ReadingTimeService.PREFIX = "About "
ReadingTimeService.PLURAL_SUFFIX = " minutes to read"

---@param words integer
---@return string
function ReadingTimeService:label(words)
  return self:_phrase(self:_minutes(words))
end

---@param words integer
---@return integer
function ReadingTimeService:_minutes(words)
  return math.max(self.ONE_MINUTE, math.floor((words + self.WORDS_PER_MINUTE - self.ONE_MINUTE) / self.WORDS_PER_MINUTE))
end

---@param minutes integer
---@return string
function ReadingTimeService:_phrase(minutes)
  if minutes == self.ONE_MINUTE then
    return self.SINGULAR
  end
  return self.PREFIX .. minutes .. self.PLURAL_SUFFIX
end
