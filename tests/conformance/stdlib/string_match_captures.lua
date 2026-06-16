local letters, digits = string.match("abc123", "(%a+)(%d+)")

return #letters, string.byte(letters, 1), #digits, string.byte(digits, 1)
