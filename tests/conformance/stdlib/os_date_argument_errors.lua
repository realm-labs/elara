local format_ok, format_message = pcall(os.date, false)
local time_ok, time_message = pcall(os.date, "!%Y", false)
local missing_field_ok, missing_field_message = pcall(os.time, { year = 1970, month = 1 })
local field_type_ok, field_type_message = pcall(os.time, {
  year = false,
  month = 1,
  day = 1,
})

return format_ok,
  string.byte(type(format_message), 1),
  time_ok,
  string.byte(type(time_message), 1),
  missing_field_ok,
  string.byte(type(missing_field_message), 1),
  field_type_ok,
  string.byte(type(field_type_message), 1)
