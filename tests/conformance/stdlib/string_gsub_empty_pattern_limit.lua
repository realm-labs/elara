local one, one_count = string.gsub("ab", "", "-", 1)
local two, two_count = string.gsub("ab", "", "-", 2)
local wide, wide_count = string.gsub("ab", "", "-", 99)

return string.len(one), string.byte(one, 1), string.byte(one, 2),
  string.byte(one, 3), one_count,
  string.len(two), string.byte(two, 1), string.byte(two, 2),
  string.byte(two, 3), string.byte(two, 4), two_count,
  string.len(wide), string.byte(wide, 1), string.byte(wide, 3),
  string.byte(wide, 5), wide_count
