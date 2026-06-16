local value = string.rep("a", 50)

return string.len(value), utf8.len(value), utf8.codepoint(value, 50),
  utf8.offset(value, 50)
