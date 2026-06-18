local result, message = io.open("__elara_missing_conformance_file_nil__.lua", nil)

return rawequal(result, nil), string.byte(type(message), 1)
