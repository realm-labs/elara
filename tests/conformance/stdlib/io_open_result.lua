local result, message = io.open("__elara_missing_conformance_file__.lua", "r")

return rawequal(result, nil), string.byte(type(message), 1)
