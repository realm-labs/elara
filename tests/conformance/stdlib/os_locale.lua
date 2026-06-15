local current = os.setlocale()
local numeric = os.setlocale("C", "numeric")
local missing = os.setlocale("elara_missing_locale")

return string.byte(current, 1), string.byte(numeric, 1), rawequal(missing, nil)
