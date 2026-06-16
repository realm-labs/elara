local start_pos, end_pos, letters, digits = string.find("abc123", "(%a+)(%d+)")

return start_pos, end_pos, #letters, string.byte(letters, 1),
  #digits, string.byte(digits, 1)
