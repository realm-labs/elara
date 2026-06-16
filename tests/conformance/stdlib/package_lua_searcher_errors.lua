local missing_ok, missing_message = pcall(package.searchers[2])
local bad_name_ok, bad_name_message = pcall(package.searchers[2], false)

return missing_ok,
  string.byte(type(missing_message), 1),
  bad_name_ok,
  string.byte(type(bad_name_message), 1)
