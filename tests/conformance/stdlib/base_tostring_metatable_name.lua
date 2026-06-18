local named = setmetatable({}, { __name = "Widget" })
local ignored = setmetatable({}, { __name = false })

local named_text = tostring(named)
local ignored_text = tostring(ignored)

return string.byte(named_text, 1),
  string.byte(named_text, 6),
  string.byte(named_text, 7),
  string.byte(named_text, 8),
  string.byte(named_text, 9),
  string.byte(ignored_text, 1),
  string.byte(ignored_text, 6)
