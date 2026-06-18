local inline = [=[alpha]=]
local skipped_newline = [=[
beta]=]

return #inline, #skipped_newline, inline < "beta", skipped_newline == "beta"
