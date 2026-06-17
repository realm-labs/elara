local literal_start, literal_end = string.find("a]b", "[]]")
local mixed_start, mixed_end = string.find("a]b", "[]a]")
local negated_nil = rawequal(string.find("]", "[^]]"), nil)
local negated_start, negated_end = string.find("a", "[^]]")
local replaced = string.gsub("a]b]", "[]]", "x")

return literal_start, literal_end, mixed_start, mixed_end,
  negated_nil, negated_start, negated_end, string.len(replaced),
  string.byte(replaced, 1), string.byte(replaced, 2),
  string.byte(replaced, 3), string.byte(replaced, 4)
