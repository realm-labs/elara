local formatted = string.format("%5d:%3i:%2d:%3u:%4o:%4x:%4X:%-5d:%-4x:%05d:%05i:%04x:%-04d",
  7, -7, "12.0", 7, 8, 255, 255, 7, 255, 7, -7, 255, 7)

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 5),
  string.byte(formatted, 8),
  string.byte(formatted, 12),
  string.byte(formatted, 20),
  string.byte(formatted, 25),
  string.byte(formatted, 30),
  string.byte(formatted, 34),
  string.byte(formatted, 39),
  string.byte(formatted, 44),
  string.byte(formatted, 50),
  string.byte(formatted, 56),
  string.byte(formatted, 64)
