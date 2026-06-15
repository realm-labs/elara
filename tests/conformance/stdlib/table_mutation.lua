local names = {"b", "d"}
local _ = table.insert(names, 1, "a")
local _ = table.insert(names, 3, "c")
local removed = table.remove(names, 2)
local joined = table.concat(names, "-")

local moved = {10, 20, 30, 40}
local _ = table.move(moved, 2, 4, 1)

local unpacked = table.unpack({7, 8, 9}, 2, 2)

return string.len(joined), string.byte(joined, 1), string.byte(removed, 1),
  moved[1], moved[2], moved[3], moved[4], unpacked
