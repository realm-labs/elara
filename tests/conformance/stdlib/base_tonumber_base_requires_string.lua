local ok, message = pcall(tonumber, 10, 2)

return ok, string.byte(type(message), 1)
