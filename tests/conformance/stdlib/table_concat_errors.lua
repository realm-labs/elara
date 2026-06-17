local bad_value_ok, bad_value_message = pcall(table.concat, { "a", false })
local bad_separator_ok, bad_separator_message = pcall(table.concat, { "a" }, false)
local bad_start_ok, bad_start_message = pcall(table.concat, { "a" }, "", false)
local bad_end_ok, bad_end_message = pcall(table.concat, { "a" }, "", 1, false)

return bad_value_ok,
  string.byte(type(bad_value_message), 1),
  bad_separator_ok,
  string.byte(type(bad_separator_message), 1),
  bad_start_ok,
  string.byte(type(bad_start_message), 1),
  bad_end_ok,
  string.byte(type(bad_end_message), 1)
