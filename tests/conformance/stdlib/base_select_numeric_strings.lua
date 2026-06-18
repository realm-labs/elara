local decimal = select("2", 10, 20, 30)
local hexadecimal = select("0x2", 10, 20, 30)
local float = select(1.9, 10, 20, 30)
local ok, message = pcall(select, "2x", 10, 20, 30)

return decimal, hexadecimal, float, ok, string.byte(type(message), 1)
