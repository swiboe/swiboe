print "Hello"



CURRENT_MODE = "normal"

always = function() return true end
in_normal_mode = function() return CURRENT_MODE == "normal" end
in_insert_mode = function() return CURRENT_MODE == "insert" end

-- -- NOCOM(#sirver): mode selection...
-- swiboe.map {
  -- keys = { "<Up>" },
  -- when = always,
  -- priority = 1000,
  -- execute = function()
     -- swiboe.call("gui.buffer_view.move_cursor", {
        -- cursor_id = swiboe.current_cursor().id(),
        -- delta = {
           -- line_index = 1,
           -- column_index = 0,
        -- },
     -- });
  -- end,
-- }

-- -- swiboe.map("d", "w", function()
   -- -- swiboe.call("gui.buffer.delete_word", {
      -- -- buffer_id = swiboe.current_buffer_view().buffer_id(),
      -- -- position = swiboe.current_cursor().position(),
   -- -- })
-- -- end)

swiboe.map {
   keys = { "i" },
   when = in_normal_mode,
   priority = 1000,
   execute = function()
      print("--> ", CURRENT_MODE)
      CURRENT_MODE = "insert"
      print("--> ", CURRENT_MODE)
   end,
}

-- swiboe.map {
   -- keys = { "i" },
   -- when = in_insert_mode,
   -- priority = 1000,
   -- execute = function()
      -- print("--> ", CURRENT_MODE)
      -- CURRENT_MODE = "normal"
      -- print("--> ", CURRENT_MODE)
   -- end,
-- }

-- swiboe.map {
   -- keys = { "<Esc>" },
   -- when = always,
   -- priority = 1000,
   -- execute = function()
      -- CURRENT_MODE = "normal"
   -- end,
-- }
