local plain = string.rep("", 3)
local separated = string.rep("", 3, ".")

return string.len(plain),
  string.len(separated),
  string.byte(separated, 1),
  string.byte(separated, 2)
