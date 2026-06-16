local result, message = io.flush()

return rawequal(result, nil), string.byte(type(message), 1)
