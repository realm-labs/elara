local formatted = os.date("!%a %A %b %B %F %T", 0)

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 5), string.byte(formatted, 14),
  string.byte(formatted, 18)
