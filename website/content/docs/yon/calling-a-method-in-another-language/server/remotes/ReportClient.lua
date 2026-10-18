-- @HTTPClient
---@class ReportClient
ReportClient = {}

---@param period string
---@return any
-- @Gateway({"class":"server/services/ReportService.py"})
function ReportClient:summarize(period)
end

---@param months integer
---@return any
-- @Gateway({"command":["python3","server/helpers/forecast.py"]})
function ReportClient:forecast(months)
end
