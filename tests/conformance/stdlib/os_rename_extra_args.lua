local result, message, code = os.rename("__elara_absent_conformance_rename_extra__", "__elara_absent_conformance_to_extra__", "ignored")

return rawequal(result, nil), string.byte(type(message), 1), code ~= 0
