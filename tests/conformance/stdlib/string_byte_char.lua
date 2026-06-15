local built = string.char(65, 66, 67)

return string.len(built), string.byte(built, 1), string.byte(built, 2),
  string.byte(built, 3), string.byte("ZA", -1)
