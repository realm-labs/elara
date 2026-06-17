os.setlocale("C", "time")

local format = "!%c|%x|%X"
local formatted = table.concat({
  os.date(format, 0),
  os.date(format, 951868799),
}, "|")

return string.len(formatted), string.byte(formatted, 1, -1)
