local date = {year = 1970, month = 1, day = 1, hour = 0, min = 0, sec = 0}
local seconds = os.time(date, "ignored", false)

return string.byte(type(seconds), 1), date.year, date.month, date.day, date.hour, date.min, date.sec
