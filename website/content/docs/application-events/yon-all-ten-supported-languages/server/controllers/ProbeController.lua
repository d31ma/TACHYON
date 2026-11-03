-- @RequestRoute("/probe")
-- @Public
---@class ProbeController
ProbeController = {
  -- @Needs
  ---@type ProbeService
  probe_service = nil,
  -- @Needs
  ---@type PublishService
  publish_service = nil,
}

---@return ProbeReady|nil
-- @GetRoute
function ProbeController:get_probe()
  return self.probe_service.last
end
