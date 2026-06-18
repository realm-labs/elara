local literal = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
local concatenated = "aaaaaaaaaaaaaaaaaaaa" .. "aaaaaaaaaaaaaaaaaaaaa"
local different = concatenated .. "b"

return literal == concatenated, literal ~= different
