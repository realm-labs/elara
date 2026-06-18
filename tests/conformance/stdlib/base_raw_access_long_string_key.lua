local key = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
local same = "aaaaaaaaaaaaaaaaaaaa" .. "aaaaaaaaaaaaaaaaaaaaa"
local values = {}

rawset(values, key, 77)

return rawget(values, same), values[same]
