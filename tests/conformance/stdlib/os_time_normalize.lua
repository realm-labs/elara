local date = {year = 1970, month = 13, day = 1}
local seconds = os.time(date)

return seconds, date.year, date.month, date.day, date.hour, date.min, date.sec
