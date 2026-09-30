-- @RequestRoute("/lifecycle")
-- @Public
---@class LifecycleController
LifecycleController = {
  -- @Needs
  ---@type AuditService
  audit_service = nil,
  -- @Needs
  ---@type CheckoutService
  checkout_service = nil,
}

---@return Lifecycle
-- @GetRoute
function LifecycleController:get_lifecycle()
  return { log = self.audit_service.log, settled = self.audit_service.settled }
end
