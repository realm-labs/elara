local selected = select(2, 10, 20, 30)
local count = select("#", 1, nil, 3)
local binary = tonumber("1010", 2)
local missing = tonumber("12x")
local text = tostring(true)

return selected, count, binary, rawequal(missing, nil), string.byte(text, 1)
