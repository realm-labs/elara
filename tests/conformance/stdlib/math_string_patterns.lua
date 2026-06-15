local start_pos = string.find("abc123def", "%d+")
local replaced = string.gsub("a1b2c3", "%d", "")
local capture = string.match("lua55", "%a+")

return math.max(2, 7, 4), math.min(2, 7, 4), math.fmod(17, 5),
  math.tointeger("42"), string.byte(math.type(3), 1), start_pos,
  string.len(replaced), string.len(capture)
