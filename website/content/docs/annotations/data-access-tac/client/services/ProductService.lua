-- @Service
---@class ProductService
ProductService = {
  -- @Needs
  ---@type ProductStore
  product_store = nil,
  ---@type string
  summary = "Not loaded.",
}

---@return nil
function ProductService:load()
  local rows = self.product_store:excluding("retired", { size = 5 })
  local active_count = self.product_store:count_by_status("active")
  self.summary = #rows .. " of " .. active_count .. " active"
end
