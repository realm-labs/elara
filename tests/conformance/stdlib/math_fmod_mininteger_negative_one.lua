local result = math.fmod(math.mininteger, -1)

return result, string.byte(math.type(result), 1)
