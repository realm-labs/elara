local result, message = io.popen("echo elara", nil)

return rawequal(result, nil), string.byte(type(message), 1)
