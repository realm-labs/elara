local chars = string.format("%3c:%-3c", 65, 66)
local pointer = string.format("%8p:%-8p", nil, nil)

return
  #chars, string.byte(chars, 1), string.byte(chars, 3),
  string.byte(chars, 4), string.byte(chars, 5),
  #pointer, string.byte(pointer, 1), string.byte(pointer, 3),
  string.byte(pointer, 9), string.byte(pointer, 15)
