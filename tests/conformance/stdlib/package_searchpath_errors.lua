local bad_name_ok, bad_name_message = pcall(package.searchpath, false, "?.lua")
local bad_path_ok, bad_path_message = pcall(package.searchpath, "mod", false)
local bad_separator_ok, bad_separator_message = pcall(package.searchpath, "mod", "?.lua", false)
local bad_directory_separator_ok, bad_directory_separator_message =
  pcall(package.searchpath, "mod", "?.lua", ".", false)

return bad_name_ok,
  string.byte(type(bad_name_message), 1),
  bad_path_ok,
  string.byte(type(bad_path_message), 1),
  bad_separator_ok,
  string.byte(type(bad_separator_message), 1),
  bad_directory_separator_ok,
  string.byte(type(bad_directory_separator_message), 1)
