print "Hello"


swiboe.call("hi")

-- map(",lb", )
-- map(", l<Space> b")

CURRENT_MODE = "normal"

always = function() return true end
in_normal_mode = function() return CURRENT_MODE == "normal" end
in_insert_mode = function() return CURRENT_MODE == "insert" end

-- -- NOCOM(#sirver): mode selection...
-- on_input {
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

-- -- map("d", "w", function()
   -- -- swiboe.call("gui.buffer.delete_word", {
      -- -- buffer_id = swiboe.current_buffer_view().buffer_id(),
      -- -- position = swiboe.current_cursor().position(),
   -- -- })
-- -- end)

-- on_input {
   -- keys = { "i" },
   -- when = in_normal_mode,
   -- priority = 1000,
   -- execute = function()
      -- CURRENT_MODE = "insert"
   -- end,
-- }

-- on_input {
   -- keys = { "<Esc>" },
   -- when = always,
   -- priority = 1000,
   -- execute = function()
      -- CURRENT_MODE = "normal"
   -- end,
-- }
