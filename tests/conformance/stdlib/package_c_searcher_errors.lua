local c_missing_ok, c_missing_message = pcall(package.searchers[3])
local c_bad_name_ok, c_bad_name_message = pcall(package.searchers[3], false)
local root_missing_ok, root_missing_message = pcall(package.searchers[4])
local root_bad_name_ok, root_bad_name_message = pcall(package.searchers[4], false)

return c_missing_ok,
  string.byte(type(c_missing_message), 1),
  c_bad_name_ok,
  string.byte(type(c_bad_name_message), 1),
  root_missing_ok,
  string.byte(type(root_missing_message), 1),
  root_bad_name_ok,
  string.byte(type(root_bad_name_message), 1)
