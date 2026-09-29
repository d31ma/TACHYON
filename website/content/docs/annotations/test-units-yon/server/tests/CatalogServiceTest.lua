-- @Test(CatalogTestSetup, {"partition":"tests"})
-- @UseProfiles("test")
---@class CatalogServiceTest
CatalogServiceTest = {}

---@param value integer
---@return integer
function CatalogServiceTest:twice(value)
  return value * 2
end

function CatalogServiceTest:test_doubles_a_value()
  Assert.equals(8, self:twice(4))
end
