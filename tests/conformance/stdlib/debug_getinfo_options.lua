local empty = debug.getinfo(1, "")
local invalid_ok = pcall(debug.getinfo, 1, "X")

return empty.currentline == nil,
  empty.what == nil,
  empty.func == nil,
  invalid_ok
