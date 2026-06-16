local concat_table_ok, concat_table_message = pcall(table.concat, false)
local sort_table_ok, sort_table_message = pcall(table.sort, false)

return concat_table_ok,
  string.byte(type(concat_table_message), 1),
  sort_table_ok,
  string.byte(type(sort_table_message), 1)
