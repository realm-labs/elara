local t = {10, 20, 30}
local packed = table.pack(100, 101)
local ok = pcall(type, nil)
local length = rawlen(t)

return string.byte(type(nil), 1), rawequal(t[1], 10), ok, length, packed.n, packed[1], packed[2]
