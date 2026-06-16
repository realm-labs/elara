local all = os.setlocale("C", "all")
local collate = os.setlocale("C", "collate")
local ctype = os.setlocale("C", "ctype")
local monetary = os.setlocale("C", "monetary")
local numeric = os.setlocale("C", "numeric")
local time = os.setlocale("C", "time")

return string.byte(all, 1), string.byte(collate, 1), string.byte(ctype, 1),
  string.byte(monetary, 1), string.byte(numeric, 1), string.byte(time, 1)
