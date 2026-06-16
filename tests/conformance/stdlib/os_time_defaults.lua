local date = {year = 1970, month = 1, day = 1, hour = 0}
local seconds = os.time(date)

return seconds, date.hour, date.min, date.sec
