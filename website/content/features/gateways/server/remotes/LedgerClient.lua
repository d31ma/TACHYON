-- @HTTPClient
---@class LedgerClient
LedgerClient = {}

---@param account string
---@return any
-- @Gateway({"class":"server/services/LedgerService.py"})
function LedgerClient:balance(account)
end

---@param months integer
---@return any
-- @Gateway({"command":["python3","server/helpers/forecast.py"]})
function LedgerClient:forecast(months)
end
