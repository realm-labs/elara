local text = string.rep("a", 50)
local literal = string.format(string.rep("x", 50))
local formatted = string.format("%s:%q:%.5s", text, text, text)

return string.len(literal), string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 51),
  string.byte(formatted, 52),
  string.byte(formatted, 53),
  string.byte(formatted, 104),
  string.byte(formatted, 109)
