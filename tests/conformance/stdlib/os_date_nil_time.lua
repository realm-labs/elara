local year = os.date("!%Y", nil, "ignored", false)

return string.byte(type(year), 1), #year, year >= "0000"
