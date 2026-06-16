local pi_type = math.type(math.pi)
local huge_type = math.type(math.huge)

return math.maxinteger + math.mininteger,
  string.byte(pi_type, 1), string.byte(huge_type, 1)
