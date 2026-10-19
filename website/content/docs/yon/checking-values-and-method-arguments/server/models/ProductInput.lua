-- @Checked
---@class ProductInput
ProductInput = {
  -- @NotEmpty
  -- @Length({"min":1,"max":80})
  ---@type string
  name = nil,
  -- @Required
  -- @Min(1)
  -- @Max(9)
  ---@type integer
  level = nil,
}
