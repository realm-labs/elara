local result, message = io.tmpfile()

return rawequal(result, nil), string.byte(type(message), 1)
