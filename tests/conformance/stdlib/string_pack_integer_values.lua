local packed = string.pack("<bBI2i2c4xz", -1, 255, 4660, -2, "hi", "ok")
local signed, unsigned, wide, negative, fixed, zero, next_pos =
  string.unpack("<bBI2i2c4xz", packed)

local values = {#packed}
for index = 1, #packed do
  values[#values + 1] = string.byte(packed, index)
end

values[#values + 1] = signed
values[#values + 1] = unsigned
values[#values + 1] = wide
values[#values + 1] = negative
values[#values + 1] = #fixed
for index = 1, #fixed do
  values[#values + 1] = string.byte(fixed, index)
end
values[#values + 1] = #zero
for index = 1, #zero do
  values[#values + 1] = string.byte(zero, index)
end
values[#values + 1] = next_pos

return table.unpack(values)
