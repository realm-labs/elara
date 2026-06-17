local packed = string.char(255, 255, 52, 18, 254, 255, 104, 105, 0, 0, 0, 111, 107, 0)
local signed, unsigned, wide, negative, fixed, zero, next_pos =
  string.unpack("<bBI2i2c4xz", packed)

local aligned = string.char(1, 0, 0, 0, 1, 2, 3, 4, 0)
local aligned_byte, aligned_integer, aligned_tail, aligned_next =
  string.unpack(">!4bI4Xdb", aligned)

local strings = string.char(3, 97, 98, 99, 111, 107, 0)
local counted, zstr, strings_next = string.unpack("<s1z", strings)
local positioned_value, positioned_next = string.unpack("B", string.char(9, 42), 2)

local short_ok, short_message = pcall(string.unpack, "I4", string.char(1, 2))
local unfinished_ok, unfinished_message = pcall(string.unpack, "z", "abc")
local position_ok, position_message = pcall(string.unpack, "I4", string.char(0, 0, 0, 0), 6)

return signed, unsigned, wide, negative,
  #fixed, string.byte(fixed, 1), string.byte(fixed, 2), string.byte(fixed, 3), string.byte(fixed, 4),
  #zero, string.byte(zero, 1), string.byte(zero, 2), next_pos,
  aligned_byte, aligned_integer, aligned_tail, aligned_next,
  #counted, string.byte(counted, 1), string.byte(counted, 2), string.byte(counted, 3),
  #zstr, string.byte(zstr, 1), string.byte(zstr, 2), strings_next,
  positioned_value, positioned_next,
  short_ok, string.byte(type(short_message), 1),
  unfinished_ok, string.byte(type(unfinished_message), 1),
  position_ok, string.byte(type(position_message), 1)
