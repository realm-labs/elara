local ok, message = pcall(string.dump, string.len)

return ok, string.byte(type(message), 1)
