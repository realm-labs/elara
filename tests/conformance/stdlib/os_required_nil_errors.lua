local getenv_ok, getenv_message = pcall(os.getenv, nil)
local remove_ok, remove_message = pcall(os.remove, nil)
local rename_from_ok, rename_from_message = pcall(os.rename, nil, "to")
local rename_to_ok, rename_to_message = pcall(os.rename, "from", nil)

return getenv_ok,
  string.byte(type(getenv_message), 1),
  remove_ok,
  string.byte(type(remove_message), 1),
  rename_from_ok,
  string.byte(type(rename_from_message), 1),
  rename_to_ok,
  string.byte(type(rename_to_message), 1)
