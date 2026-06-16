local x = 0

local function set(value)
  x = value
end

local function read()
  return x
end

set(42)

return read()
