local callable

local function call(self, first, second)
  return rawequal(self, callable), first + second
end

callable = setmetatable({}, { __call = call })

return pcall(callable, 20, 22)
