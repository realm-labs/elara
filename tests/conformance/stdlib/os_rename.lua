local result, message, code = os.rename("__elara_absent_conformance_rename__", "__elara_absent_conformance_to__")

return rawequal(result, nil), string.byte(type(message), 1), code ~= 0
