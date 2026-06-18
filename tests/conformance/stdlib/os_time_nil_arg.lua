local current = os.time(nil, "ignored", false)

return string.byte(type(current), 1), current >= 0, math.floor(current) == current
