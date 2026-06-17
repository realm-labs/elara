local date = {year = 2024, month = 2, day = 29, hour = 23, min = 59, sec = 58}
local seconds = os.time(date)

return seconds, date.year, date.month, date.day, date.hour, date.min, date.sec
