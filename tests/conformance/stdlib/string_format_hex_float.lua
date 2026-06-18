local basic = string.format("%a:%A:%a:%a", 12.5, 12.5, 0.0, -0.0)
local precision = string.format("%.0a:%#.0a:%.3a:%.3A", 12.5, 12.5, 12.5, 12.5)

return
  string.len(basic),
  string.byte(basic, 2),
  string.byte(basic, 11),
  string.byte(basic, 15),
  string.byte(basic, 26),
  string.byte(basic, 32),
  string.len(precision),
  string.byte(precision, 3),
  string.byte(precision, 11),
  string.byte(precision, 20),
  string.byte(precision, 23),
  string.byte(precision, 28),
  string.byte(precision, 34)
