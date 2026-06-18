local callable

local function call(self, first, second)
  return rawequal(self, callable), first + second
end

local function handler(message)
  return message
end

callable = setmetatable({}, { __call = call })

return xpcall(callable, handler, 20, 22)
