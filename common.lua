--	(C) Copyright 2018
--	Stefano Babic, DENX Software Engineering, sbabic@denx.de.
--
--	SPDX-License-Identifier:     GPL-2.0-or-later
--
-- Set default values, they can be overwritten
-- in the config.lua file
-------------------------------------------------------------------------------
--	Create Window common Header
-------------------------------------------------------------------------------
local ui = require "tek.ui"

RescueGUIHeader = ui.Group:newClass { _NAME = "_guiheader"}

function RescueGUIHeader.new(class, self)
	self = self or { }
	self.Class = "guiheader"
  grp = ui.Group.new(class, self)
  grp.Orientation = "vertical"
  self.title = ui.Text:new
  {
    Text = self.Description or "",
  }
    
  self.logo =  ui.ImageWidget:new 
  {
    Width = "fill",
    Height = "fill",
    Mode = "inert",
    Image = self.Image,
    HAlign = "center",
  }
  grp:addMember(self.title)
  grp:addMember(self.logo)
  
  return grp
end
