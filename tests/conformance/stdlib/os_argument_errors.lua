local difftime_ok, difftime_message = pcall(os.difftime, 1)
local getenv_ok, getenv_message = pcall(os.getenv)
local remove_ok, remove_message = pcall(os.remove, false)
local rename_ok, rename_message = pcall(os.rename, "from", false)
local setlocale_ok, setlocale_message = pcall(os.setlocale, "C", false)
local time_ok, time_message = pcall(os.time, false)

return difftime_ok,
  string.byte(type(difftime_message), 1),
  getenv_ok,
  string.byte(type(getenv_message), 1),
  remove_ok,
  string.byte(type(remove_message), 1),
  rename_ok,
  string.byte(type(rename_message), 1),
  setlocale_ok,
  string.byte(type(setlocale_message), 1),
  time_ok,
  string.byte(type(time_message), 1)
