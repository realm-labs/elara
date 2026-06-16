local bad_comparator_ok, bad_comparator_message = pcall(table.sort, { 2, 1 }, 1)
local bad_values_ok, bad_values_message = pcall(table.sort, { false, 1 })

return bad_comparator_ok,
  string.byte(type(bad_comparator_message), 1),
  bad_values_ok,
  string.byte(type(bad_values_message), 1)
