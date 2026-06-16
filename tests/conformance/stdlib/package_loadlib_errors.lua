local missing_library_ok, missing_library_message = pcall(package.loadlib)
local missing_function_ok, missing_function_message = pcall(package.loadlib, "missing.so")
local bad_library_ok, bad_library_message = pcall(package.loadlib, false, "luaopen_missing")
local bad_function_ok, bad_function_message = pcall(package.loadlib, "missing.so", false)

return missing_library_ok,
  string.byte(type(missing_library_message), 1),
  missing_function_ok,
  string.byte(type(missing_function_message), 1),
  bad_library_ok,
  string.byte(type(bad_library_message), 1),
  bad_function_ok,
  string.byte(type(bad_function_message), 1)
