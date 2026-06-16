local bad_first_ok, bad_first_message = pcall(table.move, {}, false, 1, 1)
local bad_last_ok, bad_last_message = pcall(table.move, {}, 1, false, 1)
local bad_target_ok, bad_target_message = pcall(table.move, {}, 1, 1, false)
local bad_destination_ok, bad_destination_message = pcall(table.move, { 1 }, 1, 1, 1, false)

return bad_first_ok,
  string.byte(type(bad_first_message), 1),
  bad_last_ok,
  string.byte(type(bad_last_message), 1),
  bad_target_ok,
  string.byte(type(bad_target_message), 1),
  bad_destination_ok,
  string.byte(type(bad_destination_message), 1)
