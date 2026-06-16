local create_ok, create_message = pcall(coroutine.create, false)
local wrap_ok, wrap_message = pcall(coroutine.wrap, false)
local resume_ok, resume_message = pcall(coroutine.resume, false)
local status_ok, status_message = pcall(coroutine.status, false)
local close_ok, close_message = pcall(coroutine.close, false)
local isyieldable_ok, isyieldable_message = pcall(coroutine.isyieldable, false)

return create_ok,
  string.byte(type(create_message), 1),
  wrap_ok,
  string.byte(type(wrap_message), 1),
  resume_ok,
  string.byte(type(resume_message), 1),
  status_ok,
  string.byte(type(status_message), 1),
  close_ok,
  string.byte(type(close_message), 1),
  isyieldable_ok,
  string.byte(type(isyieldable_message), 1)
