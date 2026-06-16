local formatted = os.date("!%Y-%m-%d %T %j %w %%", 0)

return string.len(formatted), string.byte(formatted, 21),
  string.byte(formatted, 25), string.byte(formatted, 27)
