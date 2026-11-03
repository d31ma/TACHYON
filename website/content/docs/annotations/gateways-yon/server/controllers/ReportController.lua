-- @RequestRoute("/report")
-- @Public
---@class ReportController
ReportController = {
  -- @Needs
  ---@type ReportClient
  report_client = nil,
}

---@return Report
-- @GetRoute
function ReportController:get_report()
  return {
    summary = self.report_client:summarize("2026-Q3"),
    forecast = self.report_client:forecast(2),
  }
end
