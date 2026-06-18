local iter, state, control = utf8.codes("A", false, "ignored")
local position, codepoint = iter(state, control)

return position, codepoint
