local ok, message = pcall(assert, false, "bad")

return ok, string.byte(type(message), 1)
