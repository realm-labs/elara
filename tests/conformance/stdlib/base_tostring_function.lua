local function worker()
end

local text = tostring(worker)

return string.byte(text, 1), string.byte(text, 9)
