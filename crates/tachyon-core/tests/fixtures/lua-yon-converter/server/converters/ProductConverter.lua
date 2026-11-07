-- @Converter
---@class ProductConverter
ProductConverter = {}

---@param product table
---@return table
-- @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
function ProductConverter:to_summary(product)
end
