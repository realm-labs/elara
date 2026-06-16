local text = string.rep("a", 50)
local literal = string.format(string.rep("x", 50))
local formatted = string.format("%s:%q:%p", text, text, text)

return string.len(literal), string.len(formatted)
