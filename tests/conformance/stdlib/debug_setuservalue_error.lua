local ok, message = pcall(debug.setuservalue, 1, 2)

return ok, string.byte(type(message), 1)
