#!/usr/bin/env lua
--	(C) Copyright 2018
--	Stefano Babic, DENX Software Engineering, sbabic@denx.de.
--
--	SPDX-License-Identifier:     GPL-2.0-or-later
--

ui = require "tek.ui"
local Button = ui.Button

local VerboseCheckMark = ui.CheckMark:newClass { _NAME = "_vbcheckmark" }
local VerboseRadioButton = ui.RadioButton:newClass { _NAME = "_vbradiobutton" }

-------------------------------------------------------------------------------
--	Create IPAddress class:
-------------------------------------------------------------------------------
IPAddress = ui.Group:newClass { _NAME = "_ipaddress"}

NetAddress = ui.Group:newClass { _NAME = "_netaddress"}

function IPAddress.getAddress(class,self)
  return self.address
end


function IPAddress.new(class, self)
	self = self or { }
	self.Class = "ipaddress"
  Orientation = "vertical"

  grp = ui.Group.new(class, self)
  self.buttons = {}
  for i = 1, 4 do
    button = ui.Button:new(addr)
    
    self.buttons[i] = ui.Button:new {
      onClick = function(self)
        val = tonumber(self.Text) + 1
        if (val > 255) then
          val = 1
        end
        self:setValue("Text", tostring(val))                    
      end
    }    
    self.buttons[i].Text= "1"
    grp:addMember(self.buttons[i])
  end
  
  return grp
end

function IPAddress:setip(address)
  local count = 1
  self.address = address
  local chunks = {address:match("(%d+)%.(%d+)%.(%d+)%.(%d+)")}
  if not (#chunks == 4) then
    return
  end
  for ipx in address:gmatch("([^.]+),?") do
    self.buttons[count]:setValue("Text", ipx)
    count = count + 1
    if (count > 4) then
        break
    end
  end
end

function IPAddress:getip()
  addr = ""
  for i = 1,4 do
    local tmp = self.buttons[i].Text
    if (i == 1) then
      addr = tmp
    else
      addr = addr .. "." .. tmp
    end
  end
  return addr
end

function IPAddress:enable(enable)
  for i = 1, 4 do
    self.buttons[i]:setValue("Disabled", not enable)
  end
end


function NetAddress.new(class, self, defaultip, defaultnetmask)
	self = self or { }
	self.Class = "netaddress"

  print(defaultip, defaultnetmask)
  self.defaultip = defaultip
  self.defaultnetmask = defaultnetmask
  self.grp = ui.Group.new(class, self)
  self.dhcp = ui.CheckMark:new
  {
    Text = "dhcp",
    Id = "dhcp",
    Selected = true,
    onSelect = function(self)
      ui.RadioButton.onSelect(self)
      self.parent:enable(self.Selected)
    end
  }
  self.dhcp.parent = self
  
  self.grp:setValue("Orientation", "vertical")
  self.grp:addMember(self.dhcp)
  
  self.ip = IPAddress:new { Legend = "IP Address"}
  self.grp:addMember(self.ip)
  self.grp:addMember(ui.Spacer:new { })
  self.netmask = IPAddress:new { Legend = "Netmask"}
  self.grp:addMember(self.netmask)
  
  return self.grp
  
end

function NetAddress:enable(dhcp)
  self.dhcp:setValue("Selected", dhcp)
  self.ip:enable(not dhcp)
  self.netmask:enable(not dhcp)
end

function NetAddress:justaddress()
  self.dhcp:setValue("Disabled", true)
  self.ip:enable(true)
  self.netmask:enable(false)
end

function NetAddress:setip(endhcp, ip, netmask, gateway)
  self.dhcp:setValue("Disabled", false)
  self.dhcp:setValue("Selected", endhcp)
  self.ip:enable(self.ip, not endhcp)
  self.netmask:enable(self.netmask, not endhcp)
  
  self.ip:setip(ip)
  self.netmask:setip(netmask)
end

function NetAddress:getip()
  return self.dhcp.Selected, self.ip:getip(), self.netmask:getip()
end
