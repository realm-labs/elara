local nil_text = tostring(nil)
local false_text = tostring(false)
local int_text = tostring(-42)

return string.byte(nil_text, 1), string.len(nil_text),
  string.byte(false_text, 1), string.len(false_text),
  string.byte(int_text, 1), string.len(int_text)
