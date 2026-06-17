local function loader()
  return nil
end

package.preload.nilmod = loader

return require("nilmod"), package.loaded.nilmod
