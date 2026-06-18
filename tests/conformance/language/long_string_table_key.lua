local key = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
local same = "aaaaaaaaaaaaaaaaaaaa" .. "aaaaaaaaaaaaaaaaaaaaa"
local different = same .. "b"
local values = {}

values[key] = 42

return values[same], values[different] == nil
