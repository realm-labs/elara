local values = { answer = 42 }

local key, value = next(values, nil, "ignored")
local done = next(values, key, "ignored")

return rawequal(key, "answer"), value, rawequal(done, nil)
