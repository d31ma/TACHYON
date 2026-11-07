-- @Setup
---@class ProductsSetup
ProductsSetup = {}

---@param execution table
---@return table
function ProductsSetup.configure(execution)
  local products = {
    exchange = function(_, request, call)
      return { items = { "anvil", "hammer" } }
    end,
  }
  return { http = { products = products } }
end
