local date = os.date("!*t", 0)

return date.wday, date.yday, date.isdst
