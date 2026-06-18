local function target(value)
  local doubled = value + value
  return doubled
end

local full = string.dump(target, false)
local stripped = string.dump(target, true)

return string.byte(type(full), 1),
  string.byte(type(stripped), 1),
  #stripped < #full
