local open_ok, open_message = pcall(io.open, nil)
local popen_ok, popen_message = pcall(io.popen, nil)

return open_ok,
  string.byte(type(open_message), 1),
  popen_ok,
  string.byte(type(popen_message), 1)
