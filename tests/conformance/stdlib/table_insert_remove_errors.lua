local insert_position_ok, insert_position_message = pcall(table.insert, {}, 2, 7)
local insert_count_ok, insert_count_message = pcall(table.insert, {}, 1, 2, 3)
local remove_position_ok, remove_position_message = pcall(table.remove, {}, 2)
local remove_count_ok, remove_count_message = pcall(table.remove, {}, 1, 2)

return insert_position_ok,
  string.byte(type(insert_position_message), 1),
  insert_count_ok,
  string.byte(type(insert_count_message), 1),
  remove_position_ok,
  string.byte(type(remove_position_message), 1),
  remove_count_ok,
  string.byte(type(remove_count_message), 1)
