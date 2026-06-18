local x = 41

local function read()
  return x
end

local name = debug.setupvalue(read, 1, 42, "ignored", false)

return string.byte(name, 1), read()
