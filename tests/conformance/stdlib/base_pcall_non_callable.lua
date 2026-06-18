local ok, message = pcall(42)

return ok, string.byte(type(message), 1)
