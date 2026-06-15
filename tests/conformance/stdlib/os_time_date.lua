local date = os.date("!*t", 0)
local next_day = os.time({year = 1970, month = 1, day = 2, hour = 0, min = 0, sec = 0})
local first_day = os.time({year = 1970, month = 1, day = 1, hour = 0, min = 0, sec = 0})

return date.year, date.month, date.day, date.hour, date.min, date.sec,
  os.difftime(next_day, first_day), string.byte(type(package.config), 1)
