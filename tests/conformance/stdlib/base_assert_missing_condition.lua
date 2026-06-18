local ok, message = pcall(assert)

return ok, string.byte(type(message), 1)
