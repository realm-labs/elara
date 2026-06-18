local ok, message = pcall(table.insert, {}, nil, 1)

return ok, string.byte(type(message), 1)
