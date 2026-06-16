local value = 12 .. 'ab'
return #value, string.byte(value, 1), string.byte(value, 2), string.byte(value, 3), string.byte(value, 4)
