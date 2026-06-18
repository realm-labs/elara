local ok, message = pcall(assert, false)
local nil_ok, nil_message = pcall(assert, false, nil)

return ok,
  string.len(message),
  string.byte(message, 1),
  string.byte(message, 10),
  string.byte(message, 17),
  nil_ok,
  nil_message == nil
