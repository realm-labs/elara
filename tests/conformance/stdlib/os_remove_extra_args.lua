local result, message, code = os.remove("__elara_absent_conformance_remove_extra__", "ignored", false)

return rawequal(result, nil), string.byte(type(message), 1), code ~= 0
