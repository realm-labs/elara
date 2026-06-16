local value = string.char(97, 0, 98, 99)
local reversed = string.reverse(value)

return string.len(reversed), string.byte(reversed, 1), string.byte(reversed, 4)
