os.setlocale("C", "time")

local format = "!%Ec|%EC|%Ex|%EX|%Ey|%EY|%Od|%Oe|%OH|%OI|%Om|%OM|%OS|%Ou|%OU|%OV|%Ow|%OW|%Oy"
local formatted = os.date(format, 951868799)

return string.len(formatted), string.byte(formatted, 1, -1)
