local bad_table_ok, bad_table_message = pcall(table.unpack, false)
local bad_start_ok, bad_start_message = pcall(table.unpack, {}, false)
local bad_end_ok, bad_end_message = pcall(table.unpack, {}, 1, false)

return bad_table_ok,
  string.byte(type(bad_table_message), 1),
  bad_start_ok,
  string.byte(type(bad_start_message), 1),
  bad_end_ok,
  string.byte(type(bad_end_message), 1)
