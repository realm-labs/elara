local ok, message = pcall(rawset, {}, nil, 42)

return ok, string.byte(type(message), 1)
