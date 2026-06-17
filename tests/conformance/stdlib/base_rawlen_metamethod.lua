local function custom_len()
  return 9
end

local values = setmetatable({ 1, 2, 3 }, { __len = custom_len })

return rawlen(values)
