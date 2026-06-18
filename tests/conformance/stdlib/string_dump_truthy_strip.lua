local function target(value)
  local doubled = value + value
  return doubled
end

local full = string.dump(target, false)
local stripped = string.dump(target, 0)

return string.byte(type(stripped), 1),
  #stripped < #full
