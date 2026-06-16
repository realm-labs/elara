local insert_table_ok, insert_table_message = pcall(table.insert, false, 1)
local insert_position_ok, insert_position_message = pcall(table.insert, {}, false, 1)
local remove_table_ok, remove_table_message = pcall(table.remove, false)
local remove_position_ok, remove_position_message = pcall(table.remove, {}, false)

return insert_table_ok,
  string.byte(type(insert_table_message), 1),
  insert_position_ok,
  string.byte(type(insert_position_message), 1),
  remove_table_ok,
  string.byte(type(remove_table_message), 1),
  remove_position_ok,
  string.byte(type(remove_position_message), 1)
