local function loader()
  return nil
end

package.preload.nilmod = loader

return package.require("nilmod"), package.loaded.nilmod
