local formatted = os.date("!%F %T", 0, "ignored", false)

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 11), string.byte(formatted, 19)
