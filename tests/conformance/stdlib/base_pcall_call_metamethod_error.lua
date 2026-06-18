local function fail()
  error(9)
end

local callable = setmetatable({}, { __call = fail })

return pcall(callable)
