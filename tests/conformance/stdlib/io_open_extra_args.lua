local result, message = io.open("__elara_missing_conformance_file_extra__.lua", "r", "ignored", false)

return rawequal(result, nil), string.byte(type(message), 1)
