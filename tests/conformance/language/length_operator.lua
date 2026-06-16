local values = { 1, 2, 3 }

local function custom_len()
  return 9
end

local custom = setmetatable({}, { __len = custom_len })

return #"abc", #values, #custom
