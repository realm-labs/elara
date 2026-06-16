local result, message = io.write("hello", 7)

return rawequal(result, nil), string.byte(type(message), 1)
