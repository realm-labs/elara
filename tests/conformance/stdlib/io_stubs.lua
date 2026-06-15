local file = io.open("__elara_missing_conformance_file__.lua", "r")
local tmp = io.tmpfile()
local typed = io.type(file)

return rawequal(file, nil), rawequal(tmp, nil), rawequal(typed, nil)
