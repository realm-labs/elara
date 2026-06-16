local formatted = os.date("!%F %T %y %j %w %h", 951868799)

return string.len(formatted), string.byte(formatted, 1, -1)
