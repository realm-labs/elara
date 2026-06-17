local formatted = os.date("!%C %D %e %I %p %r %R %n%t", 951868799)

return string.len(formatted), string.byte(formatted, 1, -1)
