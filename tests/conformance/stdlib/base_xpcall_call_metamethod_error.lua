local function fail()
  error(9)
end

local function handler(message)
  return message + 1
end

local callable = setmetatable({}, { __call = fail })

return xpcall(callable, handler)
