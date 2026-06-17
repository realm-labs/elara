local calls = 0

local function trap()
  calls = calls + 1
end

local values = setmetatable({}, { __newindex = trap })
local written = rawset(values, "name", 42)

return rawequal(written, values), rawget(values, "name"), calls
