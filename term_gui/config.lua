-- Copyright (c) The Swiboe development team. All rights reserved.
-- Licensed under the Apache License, Version 2.0. See LICENSE.txt
-- in the project root for license information.

JSON = (loadfile "json.lua")()


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
   -- when = in_normal_mode,
   -- priority = 1000,
   execute = function()
      print("--> ", CURRENT_MODE)
      CURRENT_MODE = "insert"
      print("--> ", CURRENT_MODE)
   end,
}

swiboe.map {
   keys = { "s" },
   -- when = in_normal_mode,
   -- priority = 1000,
   execute = function(client)
      local args = JSON:encode_pretty {
          foo = "blah",
          b = {
             c = 2,
             d = "blub",
          },
       }
      client:call("test.test", args);
   end,
}

swiboe.map {
   keys = { "<Up>" },
   -- when = in_normal_mode,
   -- priority = 1000,
   execute = function(client)
      -- NOCOM(#sirver): need to implement our own lua table to JSON converter eventually.
      local args = JSON:encode_pretty {
          -- NOCOM(#sirver): for now, we move any cursor, but we need to expose
          -- which cursor should be moved somehow for this method.
          cursor_id = "every_cursor_currently_has_this_id",
          delta = { line_index = -1, column_index = 0, },
       }
       -- NOCOM(#sirver): this should return an RPC object, but for now, we just implicitly wait.
       client:call("gui.buffer_view.move_cursor", args);
   end,
}

swiboe.map {
   keys = { "<Down>" },
   -- when = in_normal_mode,
   -- priority = 1000,
   execute = function(client)
      -- NOCOM(#sirver): need to implement our own lua table to JSON converter eventually.
      local args = JSON:encode_pretty {
          -- NOCOM(#sirver): for now, we move any cursor, but we need to expose
          -- which cursor should be moved somehow for this method.
          cursor_id = "every_cursor_currently_has_this_id",
          delta = { line_index = 1, column_index = 0, },
       }
       -- NOCOM(#sirver): this should return an RPC object, but for now, we just implicitly wait.
       client:call("gui.buffer_view.move_cursor", args);
   end,
}

swiboe.map {
   keys = { "<Left>" },
   -- when = in_normal_mode,
   -- priority = 1000,
   execute = function(client)
      -- NOCOM(#sirver): need to implement our own lua table to JSON converter eventually.
      local args = JSON:encode_pretty {
          -- NOCOM(#sirver): for now, we move any cursor, but we need to expose
          -- which cursor should be moved somehow for this method.
          cursor_id = "every_cursor_currently_has_this_id",
          delta = { line_index = 0, column_index = -1, },
       }
       -- NOCOM(#sirver): this should return an RPC object, but for now, we just implicitly wait.
       client:call("gui.buffer_view.move_cursor", args);
   end,
}

swiboe.map {
   keys = { "<Right>" },
   -- when = in_normal_mode,
   -- priority = 1000,
   execute = function(client)
      -- NOCOM(#sirver): need to implement our own lua table to JSON converter eventually.
      local args = JSON:encode_pretty {
          -- NOCOM(#sirver): for now, we move any cursor, but we need to expose
          -- which cursor should be moved somehow for this method.
          cursor_id = "every_cursor_currently_has_this_id",
          delta = { line_index = 0, column_index = 1, },
       }
       -- NOCOM(#sirver): this should return an RPC object, but for now, we just implicitly wait.
       client:call("gui.buffer_view.move_cursor", args);
   end,
}

MIN = -2147483648
MAX = 2147483647

swiboe.map {
   keys = { "g", "g" },
   -- when = in_normal_mode,
   -- priority = 1000,
   execute = function(client)
      -- NOCOM(#sirver): need to implement our own lua table to JSON converter eventually.
      local args = JSON:encode_pretty {
          -- NOCOM(#sirver): for now, we move any cursor, but we need to expose
          -- which cursor should be moved somehow for this method.
          cursor_id = "every_cursor_currently_has_this_id",
          delta = { line_index = MIN, column_index = MIN, },
       }
       -- NOCOM(#sirver): this should return an RPC object, but for now, we just implicitly wait.
       client:call("gui.buffer_view.move_cursor", args);
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
