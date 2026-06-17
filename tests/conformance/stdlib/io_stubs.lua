local file = io.open("__elara_missing_conformance_file__.lua", "r")
local typed = io.type(file)
local non_file_type = io.type(1)

return rawequal(file, nil), rawequal(typed, nil), rawequal(non_file_type, nil)
