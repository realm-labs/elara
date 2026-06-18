local first
local second

local function call(self, original, value)
  return rawequal(self, first), rawequal(original, second), value + 1
end

local function handler(message)
  return message
end

first = setmetatable({}, { __call = call })
second = setmetatable({}, { __call = first })

return xpcall(second, handler, 41)
