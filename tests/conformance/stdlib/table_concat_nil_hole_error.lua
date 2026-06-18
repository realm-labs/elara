local ok, message = pcall(table.concat, { "a", nil, "c" }, "", 1, 3)

return ok, string.byte(type(message), 1)
