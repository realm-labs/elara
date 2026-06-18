local ok, message = pcall(select, -4, 10, 20, 30)

return ok, string.byte(type(message), 1)
