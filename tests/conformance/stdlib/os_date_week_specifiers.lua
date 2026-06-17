local format = "!%Y-%m-%d %u %U %W %V %G %g"
local formatted = table.concat({
  os.date(format, 1709251198),
  os.date(format, 1609459200),
  os.date(format, 1672531199),
}, "|")

return string.len(formatted), string.byte(formatted, 1, -1)
