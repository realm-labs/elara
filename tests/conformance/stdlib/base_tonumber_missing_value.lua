local ok, message = pcall(tonumber)

return ok, string.byte(type(message), 1)
