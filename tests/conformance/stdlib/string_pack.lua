local packed = string.pack("<bBI2i2c4xz", -1, 255, 4660, -2, "hi", "ok")
local aligned = string.pack(">!4bI4Xdb", 1, 16909060, 0)

local values = {#packed}
for index = 1, #packed do
  values[#values + 1] = string.byte(packed, index)
end

values[#values + 1] = #aligned
for index = 1, #aligned do
  values[#values + 1] = string.byte(aligned, index)
end

local signed_ok, signed_message = pcall(string.pack, "b", 128)
local unsigned_ok, unsigned_message = pcall(string.pack, "B", 256)
local char_ok, char_message = pcall(string.pack, "c1", "ab")
local zero_ok, zero_message = pcall(string.pack, "z", string.char(97, 0, 98))

values[#values + 1] = signed_ok
values[#values + 1] = string.byte(type(signed_message), 1)
values[#values + 1] = unsigned_ok
values[#values + 1] = string.byte(type(unsigned_message), 1)
values[#values + 1] = char_ok
values[#values + 1] = string.byte(type(char_message), 1)
values[#values + 1] = zero_ok
values[#values + 1] = string.byte(type(zero_message), 1)

return table.unpack(values)
