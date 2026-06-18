local x = 41

local function read()
  return x + 1
end

local name, value = debug.getupvalue(read, 1, "ignored", false)

return string.byte(name, 1), value, read()
