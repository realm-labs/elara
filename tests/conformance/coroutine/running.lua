local thread, is_main = coroutine.running()

return string.byte(type(thread), 1), is_main
