local function probe(a, b)
  return a + b
end

local first = debug.getlocal(probe, 1, "ignored")
local second = debug.getlocal(probe, 2, false)
local missing = debug.getlocal(probe, 3, "ignored")

return string.byte(first, 1), string.byte(second, 1), missing == nil
