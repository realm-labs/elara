local t = {10, 30}
table.insert(t, "2.0", 20)
local removed = table.remove(t, "0x2")
local unpack_first, unpack_second = table.unpack(t, 1.0, "0x2")
local moved = {}
table.move(t, "1", "2.0", "0x2", moved)
local concat = table.concat({"a", "b", "c"}, "-", "2.0", "0x3")
local bad_remove_ok, bad_remove_message = pcall(table.remove, t, "1.5")

return t[1], t[2], removed, unpack_first, unpack_second,
  moved[2], moved[3], string.len(concat), string.byte(concat, 1),
  string.byte(concat, 2), string.byte(concat, 3), bad_remove_ok,
  string.byte(type(bad_remove_message), 1)
