-- @Converter
ProductConverter = {}
---@param product Product
---@return Summary
-- @Convert({"fields":[{"target":"kind","constant":"product"}]})
function ProductConverter:map(product)
end
---@param product Product
---@return Product
function ProductConverter:copy(product)
end
