#!/usr/bin/env lua

--	GUI for SWUPdate Rescue mode
--	
--	(C) Copyright 2018 - 2019
--	Stefano Babic, DENX Software Engineering, sbabic@denx.de.
--
--	SPDX-License-Identifier:     GPL-2.0-or-later
--
-- This is a simple GUI for the SWUpdate tool
-- and it is thought to be used when SWUpdate runs
-- as rescue system.
-- 

-- check for required not standard modules
-- lfs is required to browse files on local media

VERSION="0.1"

STATUS_IDLE = 0
STATUS_START = 1
STATUS_RUN = 2
STATUS_SUCCESS = 3
STATUS_FAILURE = 4
STATUS_DONE = 6

-- interface name for default Gateway
INTF_GATEWAY = "Gateway"

-- Check for components outside this project
-- they must be available, else an error is reported
local lfs
pcall(function() lfs = require "lfs" end)
if not lfs then
  print "This program requires the luafilesystem library."
  return
end
local sw = require "lua_swupdate"
if not sw then
  print "This program requires the lua swupdate library."
  return
end

-- Load other modules of the project
require "common"
require "SWUpdateGUI_ipaddress"
require "SWUpdateGUI_osinterface"

-- This is the interface to the tekui library
-- Load modules that are used in this application
local ui = require "tek.ui"
local List = require "tek.class.list"
local Button = ui.Button
local ImageWidget = ui.ImageWidget
local Group = ui.Group
local Text = ui.Text
local exec = require "tek.lib.exec"
local Gauge = ui.Gauge
local db = require "tek.lib.debug"
local rdargs = require "tek.lib.args".read

-- Set default values, they can be overwritten
-- in the config.lua file
APP_ID = "SWUpdate-GUI"
VENDOR = "SWUpdate"
LOGO = "images/logo.png"
LANG = "en"
MEDIA = "/media"
MEDIAPATH = "/media"
SCANLEVEL = -1 -- scan recursive all directories

numswusinupd = 1
automaticreboot = false

db.level = db.INFO

-- TODO: is it necessary or better use just config.lua ?
local ARGTEMPLATE = "-r=rotate/N,--help=HELP/S,-l=LOCALE/S"
local args = rdargs(ARGTEMPLATE, arg)
if not args or args.help then
  print(ARGTEMPLATE)
  return
end

function lfs.readdir(path)
  local dir, iter = lfs.dir(path)
  return function()
    local e
    repeat
      e = dir(iter)
    until e ~= "." and e ~= ".."
    return e
  end
end

-- Try to load configuration file
-- if any
local config
pcall(function() config = require "config" end)

local L = ui.getLocale("SWUpdate-GUI", VENDOR, "en", LANG)
LogoImage = ui.loadImage(LOGO) 

interfaces = { }

-- This is taken from the book
-- Programming LUA, 4nd edition
function pairsByKeys (t, f)
  local a = {}
  for n in pairs(t) do table.insert(a, n) end
  table.sort(a, f)
  local i = 0      -- iterator variable
  local iter = function ()   -- iterator function
    i = i + 1
    if a[i] == nil then return nil
    else return a[i], t[a[i]] end
  end
  return iter
end

function GetFileName(url)
  return url:match("^.+/(.+)$")
end

function GetFileExtension(url)
  return url:match("[^.]+$" )
end

-------------------------------------------------------------------------------
--	Find the local interface and add widgets to show the IP addresses
-------------------------------------------------------------------------------
--
-- Getting default gateway

function convertip(ip)
  if not (#ip == 8) then return nil end
  val = string.format("%d.%d.%d.%d", tonumber("0x" .. string.sub(ip,7,8)),
    tonumber("0x" .. string.sub(ip,5,6)),
    tonumber("0x" .. string.sub(ip,3,4)),
    tonumber("0x" .. string.sub(ip,1,2)))
  return val
end

function getgatewayip()
  local f = assert(io.open("/proc/net/route", "r"))
  for entry in f:lines() do
    local intf, dest, gw = string.match(entry, "(%g+)%s+(%g+)%s+(%g+)")
    if (dest == "00000000") then
      f:close()
      return convertip(gw)
    end
  end
  f:close()
  return nil
end

-- Update an entry in the interface table

function findintf(name)
  for _, t in pairs(interfaces) do
    if t["name"] == name then
      return t
    end
  end
  return nil
end

function modinterface(name, dhcp, addr, netmask)
  t = findintf(name)
  if t then
    t["dhcp"] = dhcp
    t["addr"] = addr
    t["netmask"] = netmask
  else
    t = {}
    t["name"] = name
    t["dhcp"] = false
    t["addr"] = addr
    t["netmask"] = netmask
    table.insert(interfaces, t)
  end
end

function updnetinterfaces()
  local netgroup = app:getById("interfaces-group")
  for _, t in pairs(interfaces) do
    if not (t["name"] == INTF_GATEWAY) then
      local netcaption = ui.Text:new
      {
        Id = "net-" .. t["name"],
        Text = t["name"] .. ": " .. t["addr"] .. " " .. t["netmask"],
        HAlign = "left",
        Height = "auto",
        Style = "text-align:left"
      }
      elem = app:getById("net-" .. t["name"])
      if elem then 
        elem:setValue("Text", t["name"] .. ": " .. t["addr"] .. " " .. t["netmask"])
      else
        netgroup:addMember(netcaption)
      end
    end
  end
end

local function loadnetinterfaces()
  -- read interface from hardware
  local readintf = sw:ipv4()
  local gw = getgatewayip()
  local runintf = {}

  if NETWORK_INTERFACES then
    for cnt=1, #NETWORK_INTERFACES do
      local name = NETWORK_INTERFACES[cnt]
      if not readintf[name] then
        -- interface not yet configured
        runintf[name] = ""
      else
        runintf[name] = readintf[name]
      end
    end
  end


  for name,v in pairsByKeys (runintf) do
    local intf = findintf(name)
    local addr,netmask = string.match(v, "(.*) (.*)")
    if not addr then addr = "" end
    if not netmask then netmask = "" end

    if not intf then
      modinterface(name, false, addr, netmask)
    else
      intf["addr"] = addr
      intf["netmask"] = netmask
    end
  end
  if gw then
    modinterface("Gateway", false, gw, "0.0.0.0")
  end

  updnetinterfaces()
end

local function searchupd(path)
  local file
  local found
  local count = 0
  if not lfs.attributes(path) then
    return nil
  end
  for iter in lfs.dir(path) do
    local file = tostring(iter)
    local ext = GetFileExtension(file)
    if ext == "upd" then
      found = file
      count = count + 1
    end
  end
  if count == 1 then
    return found
  end 
  return nil
end

function isDir(name)
    if type(name)~="string" then return false end
    local cd = lfs.currentdir()
    local is = lfs.chdir(name) and true or false
    lfs.chdir(cd)
    return is
end

local function rescan(path, t, level)
  local file
  local found
  local count = 0
  local fullrecursive = false
  if not t then
    t = {}
  end
  db.trace("SCAN PATH :" ..  path)
  if not lfs.attributes(path) or not lfs.attributes(path).mode == "directory" then
    return t
  end
  if level == -1 then
    fullrecursive = true
  end
  if not isDir(path) then
    return t
  end
  
  for iter in lfs.dir(path) do
    local file = tostring(iter)
    if iter ~= ".." and iter ~= "." then
      file = path .. "/" .. file
      if lfs.attributes(file) then
        if lfs.attributes(file).mode == "directory" then
          if fullrecursive then
             t = rescan(file, t, - 1)
          elseif level > 0 then
             t = rescan(file, t, level - 1)
          end
          ::continue::        
        end
      end
      local ext = GetFileExtension(file)
      if ext then
        ext = string.lower(ext)
        if ext == "upd" or ext == "swu" then
          db.info("SCAN : ADDED " .. file)
          table.insert(t,file)
        end
      end
    end
  end
  return t
end


local function updtoswulist(path, updfile)
  local cnt = 1
  list = {}
  for line in io.lines(path .. "/" .. updfile) do
      -- take first word
    for w in string.gmatch(line, "%g+") do
       list[cnt] = path .. "/" .. w
       break
    end
    cnt = cnt + 1
  end
  return list
end


-------------------------------------------------------------------------------
--	Progress Window
-------------------------------------------------------------------------------

local progwin = ui.Window:new
{
  Id = "progress-window",
  Title = APP_ID .. " " .. VERSION,
  Orientation = "vertical",
  Status = "hide",  
  HideOnEscape = true,
  SizeButton = false,
  FullScreen = true,
  show = function(self)
    ui.Window.show(self)
    self.Window:addInputHandler(ui.MSG_KEYDOWN, self, self.keypressed)
  end,
  hide = function(self)
    ui.Window.hide(self)
    self.Window:remInputHandler(ui.MSG_KEYDOWN, self, self.keypressed)
  end,
  keypressed = function(self)
    self.Window:hide() 
    w = self:getById("MainWindow")
    w:setValue("Status", "show")
  end,
  Children = 
  {
    RescueGUIHeader:new 
    { 
      Description = APP_ID .. " " .. VERSION,
      Image = LogoImage,
    },
    ui.Text:new
    {
      Class = "caption",
      Id = "progress-caption",
      Style = "border-style:ridge; border-rim-width: 1; border-focus-width: 1; border-width: 4;",
      Text = L.UPDATE_IN_PROGRESS,
    },

    Group:new
    {
      Orientation = "vertical",
      Children =
      {
        Gauge:new
        {
          Legend = L.NUMBER_OF_STEPS,
          Min = 0,
          Max = 100,
          Height = 200,
          Id = "slider-steps",
          Text = "Progress"
        },
        Gauge:new
        {
          Legend = L.CURRENT_STEP,
          Min = 0,
          Max = 100,
          Height = 200,
          Id = "slider-step",
          Text = "Progress1"
        },

        ui.ScrollGroup:new
        {
          Legend = L.OUTPUT,
          VSliderMode = "auto",
          Child = ui.Canvas:new
          {
            AutoWidth = true,
            Child = ui.FloatText:new
            {
              Id = "message-box",
            }
          }
        },
        ui.Button:new
        {
          Id = "progress-cancel-button",
          Text = L.BACK,
          Disabled = true,
          onClick = function(self)
            app:switchwindow("MainWindow")
          end
        }
      }
    }
  }
}

-------------------------------------------------------------------------------
--	Network Window
-------------------------------------------------------------------------------

local NetWindow = ui.Window:newClass { _NAME = "_network_window" }

function NetWindow:setRecord(fields)
  local net = self:getById("network-fields")
  local dhcp = false
  if fields[2] == "yes" then
    dhcp = true
  end
  checkip = function(ip, default)
    local chunks = {ip:match("(%d+)%.(%d+)%.(%d+)%.(%d+)")}
    if (#chunks == 4) then
      return ip
    else
      return default
    end
  end

  net:setip(dhcp,checkip(fields[3], "192.168.0.1"), checkip(fields[4], "255.255.255.0"))
  net:enable(dhcp)
  if (fields[1] == INTF_GATEWAY) then
    net:setip(dhcp,checkip(fields[3], "192.168.0.1"), "0.0.0.0")
    net:justaddress()
  end

end

function NetWindow:loadinterfaces()
  local list = self:getById("network-list")
  for _, t in pairs(interfaces) do
    local intf = { }
    table.insert(intf, t["name"])
    if not t["dhcp"] then 
      table.insert(intf, "no")
    else
      table.insert(intf, "yes")      
    end
    table.insert(intf,t["addr"])
    table.insert(intf, t["netmask"])
    local item = {}
    table.insert (item, intf)
    list:addItem(item)
  end
end

local netwin = NetWindow:new
{
  Id = "network-window",
  Title = APP_ID .. " " .. VERSION,
  Orientation = "vertical",
  Status = "hide",  
  HideOnEscape = true,
  SizeButton = true,
  FullScreen = true,
  hide = function(self)
    ui.Window.hide(self)
    app:switchwindow("MainWindow")
  end,
  Children = 
  {
    RescueGUIHeader:new 
    { 
      Description = APP_ID .. " " .. VERSION,
      Image = LogoImage,
    },
    ui.ListView:new
    {
      VSliderMode = "auto",
      HSliderMode = "auto",
      Headers = {L.NETIF, L.DHCP, L.IP_ADDRESS, L.NETMASK},
      Child = ui.Lister:new
      {
        Id = "network-list",
        SelectMode = "single",
        SelectedLine = 1,
        InitialFocus = true,
        ListObject = List:new
        {
          Id = "network-objects",
          Items = {}
        },
        onSelectLine = function(self)
          ui.Lister.onSelectLine(self)
          local line = self:getItem(self.SelectedLine)
          if line then
            self.Window:setRecord(line[1])
          end
          -- check for gateway and disable dhcp
        end,
      }
    },
    NetAddress:new
    {
      Id = "network-fields"
    },
    ui.Group:new
    {
      Orientation = "horizontal",
      SameSize = true,
      Children =
      {
        ui.Button:new 
        { 
          Id = "apply-button", 
          Text = L.APPLY,
          onClick = function(self)
            ui.Button.onClick(self)
            local list = self:getById("network-list")
            local line = list:getItem(list.SelectedLine)
            if not line then
              print("No interface selected, no change")
              return
            end
            local intf
            local name = ""
            if (line) then
              intf = line[1]
              name = intf[1]
            end
            net = self:getById("network-fields")
            local dhcp, ip, netmask = net:getip()
            modinterface(name, dhcp, ip, netmask)

            -- Call OS to setup the network interfaces
            ifup(interfaces)

            -- Reload values from kernel to check if they were set
            loadnetinterfaces()
            local t = findintf(name)
            if t then
              line = {}
              table.insert (line, name)
              if t["dhcp"] then
                table.insert (line, "yes")
              else
                table.insert (line, "no")
              end
              table.insert (line, t["addr"])
              table.insert (line, t["netmask"])
              local newval = { line }
              list:changeItem(newval, list.SelectedLine)
              list:rethinkLayout(true, 1)
            end

            if SAVEIPADDRESS then
              app:addCoroutine(function()
                  result = app:easyRequest(false, L.CONFIRM_SAVE_NETWORK, L.SAVE, L.CANCEL)
                  if result == 1 then
                    SAVEIPADDRESS(interfaces)
                  end
                end)
            end
          end
        },
        ui.Button:new 
        { 
          Id = "cancel-button", 
          Text = L.CANCEL,
          onClick = function(self)
            app:switchwindow("MainWindow")
          end
        }
      }
    }
  }
}

-------------------------------------------------------------------------------
--	Filebox Window
-------------------------------------------------------------------------------

local FileboxWindow = ui.Window:newClass { _NAME = "_filebox_window" }

function FileboxWindow:scanmedia()
  files = rescan(MEDIA, nil, SCANLEVEL)
  if files then
    db.trace ("Elements found: ", #files)
  end
  self.filelist = files
  local list = self:getById("files-list")
  list:clear()
  for i=1,#files do
      local entry = {}
      table.insert(entry, GetFileName(files[i]))
      local size = lfs.attributes(files[i]).size
      table.insert(entry, size)
      local item = {}
      table.insert(item, entry)
      list:addItem(item)
  end
end

local filebox = FileboxWindow:new
{
  Id = "filebox-window",
  Title = APP_ID .. " " .. VERSION,
  Orientation = "vertical",
  Status = "hide",  
  HideOnEscape = true,
  SizeButton = false,
  --FullScreen = true,
  hide = function(self)
    ui.Window.hide(self)
    app:switchwindow("MainWindow")
  end,
  show = function(self)
    ui.Window.show(self)
    self:scanmedia()
    local list = self:getById("files-list")
    list:repaint()
  end,
  Children = 
  {
    RescueGUIHeader:new 
    { 
      Description = APP_ID .. " " .. VERSION,
      Image = LogoImage,
    },
    ui.ListView:new
    {
      VSliderMode = "on",
      HSliderMode = "off",
      Headers = {L.FILENAME, L.SIZE},
      Child = ui.Lister:new
      {
        Id = "files-list",
        SelectMode = "single",
        SelectedLine = 1,
        InitialFocus = true,
        ListObject = List:new
        {
          Id = "files-objects",
          Items = {}
        },
        onSelectLine = function(self)
          ui.Lister.onSelectLine(self)
          local line = self:getItem(self.SelectedLine)
          if line then
          end
        end,
      }
    },
    ui.Group:new
    {
      Orientation = "horizontal",
      SameSize = true,
      Children =
      {
        ui.Button:new 
        { 
          Id = "install-button", 
          Text = L.START,
          onClick = function(self)
            ui.Button.onClick(self)
            local list = self:getById("files-list")
            local index = list.SelectedLine
            local line = list:getItem(index)
            local swulist = {}
            if not line then
              print("No file selected, no install")
              return
            end
            local file = self.Window.filelist[index]
            local ext = GetFileExtension(file)
            local basepath = string.gsub(file, "(.*/)(.*)", "%1")
            if ext == "upd" then
              swulist = updtoswulist(basepath, GetFileName(file))
              numswusinupd = #swulist
            else
              swulist[1] = file
            end
            app:addCoroutine(function()
                app:sendswu(swulist)
            end)        
          end
        },
        ui.Button:new 
        { 
          Id = "rescan-button", 
          Text = L.RESCAN,
          onClick = function(self)
            self.Window:scanmedia()
          end
        },

        ui.Button:new 
        { 
          Id = "abort-button", 
          Text = L.CANCEL,
          onClick = function(self)
            app:switchwindow("MainWindow")
          end
        },
        ui.Text:new
        {
          Id = "dummy-filebox",
          Disabled = "true",
          Style = [[
            border-width : 0;
          ]]
        }
      }
    }
  }
}

-------------------------------------------------------------------------------
--	Application:
-------------------------------------------------------------------------------

app = ui.Application:new
{
  ProgramName = "SWUpdate Rescue GUI",
  Author = "Stefano Babic",
  Copyright = "Copyright © 2018, Stefano Babic",
  ApplicationId = APP_ID,
  AuthorStyleSheets = STYLESHEETS,

  -- This can be used in future to add some setup
  SWUpdateCfg = {

  },

  setup = function(self)
    ui.Application.setup(self)
    self.Application:addInputHandler(ui.MSG_USER, self, self.msgUser)
  end,
  cleanup = function(self)
    ui.Application.cleanup(self)
    self.Application:remInputHandler(ui.MSG_USER, self, self.msgUser)
  end,
  msgUser = function(self,msg)
    local prog = msg[-1]
    self:switchtoprog(self)
    g = self:getById("progress-cancel-button")
    g:setValue("Disabled", true)
    t = {}
    for field in string.gmatch(prog, "%S+" ) do
      k,v = string.match(field, "(%a+)=\'(.*)\'")
      if k then
        t[k] = v
        db.trace (k .. " = " .. v)
      end
    end
    self:updateProgress(t)    
  end,
  switchwindow = function(self, newwin)
    local windows = {"MainWindow", "progress-window", "network-window", "filebox-window"}
    for i=1, #windows do
      win = windows[i]
      w = self:getById(win)
      if (not w) then
        db.error("Window " .. win, " .. Not found !!")
        return
      end
      if win ~= newwin then
        w:setValue("Status", "hide")
      end
    end
    w = self:getById(newwin)
    w:setValue("Status", "show")    
  end,
  switchtoprog = function(self)
    self:switchwindow("progress-window")
  end,
  sendswu = function(self, list)
    local cmd = "swupdate-client "
    for i=1, #list do
      cmd = cmd .. list[i] .. " "
    end
    if string.len(cmd) > 0 then
      cmd = cmd .. " &"
      print ("SWUSEND", cmd)
      os.execute(cmd)
    end
  end,
  reboot_device = function (self)
    if not NOREBOOT then
      self:addCoroutine(function()
        result = self:easyRequest(false, L.REBOOTING)
        end)
      os.execute ("/sbin/reboot")
    else
      print("Just for test, reboot simulated")
    end
  end,
  updateProgress = function(self, prog)
    if not prog then return end
    for k,v in pairs(prog) do
      print(k,v)
    end
    g = self:getById("slider-steps")
    g:setValue("Max", tonumber(prog["nsteps"]))
    g:setValue("Value", tonumber(prog["step"]))
    g = self:getById("slider-step")
    g:setValue("Value", tonumber(prog["percent"]))

    msg = self:getById("message-box")
    msg:setValue("Text", prog["artifact"])
    status = tonumber(prog["status"])

    g = self:getById("progress-caption")

    if status == STATUS_RUN then
      g:setValue("Text", L.UPDATE_IN_PROGRESS)
      g:setValue("Style", "color: #000000;")
    elseif status == STATUS_FAILURE then
      g:setValue("Text", L.FAILURE)
      g:setValue("Style", "color: #ff0000;")
      g = self:getById("progress-cancel-button")
      g:setValue("Disabled", false)
      numswusinupd = 1
      automaticreboot = false
    elseif status == STATUS_SUCCESS then
      g:setValue("Text", L.SUCCESS)
      g:setValue("Style", "color: #00ff00;")
      g = self:getById("progress-cancel-button")
      numswusinupd = numswusinupd - 1
      print("numswusinupd :", numswusinupd)
      if numswusinupd == 0 then
        g:setValue("Disabled", false)
        if automaticreboot then
          self:reboot_device(self)
        end
      end
    elseif status == STATUS_START then
      g:setValue("Text", L.STARTING_UPDATE)
      g:setValue("Style", "color: #000000;")
    else

    end
  end,
  
  Children =
  {
    ui.Window:new
    {
      Orientation = "vertical",
      Id = "MainWindow",
      show = function(self)
        ui.Window.show(self)
        self.Window:addInputHandler(ui.MSG_KEYDOWN, self, self.keypressed)
      end,
      hide = function(self)
        ui.Window.hide(self)
        self.Window:remInputHandler(ui.MSG_KEYDOWN, self, self.keypressed)
      end,

      -- In MainWindow, it is just possible to iterate the menu
      keypressed = function(self, msg)
        local fe = self.FocusElement
        local key = msg[3]
        local qual = msg[6]
        local retrig = key ~= 0 and self.Application:setLastKey(key)
        self:setValue("Active", true)

        if qual == 0 and (key == 13 or key == 32) and not retrig then
          if fe and not fe.Active then
            self:setHiliteElement(fe)
            self:setActiveElement(fe)
          end
        elseif (key == 9 and qual == 0) or key == 61459 then
          self:setFocusElement(self:getNextElement(fe))
        elseif (key == 9 and (qual >=1 and qual <= 3)) or key == 61458 then
          self:setFocusElement(self:getNextElement(fe, true))
        end
      end,

      Children =
      {
        RescueGUIHeader:new 
        { 
          Description = APP_ID .. " " .. VERSION,
          Image = LogoImage,
        },

        Group:new
        {
          Orientation = "vertical",
          HAlign = "center",
          Children =
          {
            Button:new
            {
              Text = L.INSTALL_FROM_FILE,
              Width = "fill",
              HAlign = "center",
              VAlign = "center",
              Height = "auto",
              InitialFocus = true,

              onClick = function(self)
                local count = 1
                local app = self.Application
                local w, h = app:getById("MainWindow").Drawable:getAttrs("WH")
                local singleupd = searchupd(MEDIAPATH)
                swulist = {}
                if singleupd then
                  print ("Found ", singleupd)
                  swulist = updtoswulist(MEDIAPATH, singleupd)
                  numswusinupd = #swulist
                  automaticreboot = true
                  app:addCoroutine(function()
                    app:sendswu(swulist)
                  end)
                else
                  app:switchwindow("filebox-window")
                end  
              end
            },
            Button:new
            {
              Text = L.NETWORK_SETUP,
              HAlign = "center",
              VAlign = "center",
              Width = "fill",
              Height = "auto",
              onClick = function(self)
                app:switchwindow("network-window")
                local g = app:getById("network-list")
                if g:getN(g) > 0 then
                  g:setValue("SelectedLine", g:getItem(1), true)
                  g:moveLine(1, true)
                end
                g = app:getById("network-fields")
                g:deactivate()
              end,
            },

            Button:new
            {
              Text = L.RESTART,
              HAlign = "center",
              Width = "fill",
              Height = "auto",
              onClick = function(self)
                app:reboot_device(app)
              end            
            }
          }
        },
        ui.Spacer:new { },
        Group:new
        {
          Id = "interfaces-group",
          Orientation = "vertical",
          HAlign = "center",
          Children =
          {

          }
        }
      }
    }
  }
}

-------------------------------------------------------------------------------
--	Progress task connecting to swupdate
-------------------------------------------------------------------------------

local progtask = exec.run(function()
    -- child task:
    local exec = require "tek.lib.exec"
    local ui = require "tek.ui"
    local sw = require "lua_swupdate"
    prog = sw:progress()
    exec.sendmsg("*p", "Test")

    while true do
      r = prog:receive()
      local tmp = ""
      for k,v in pairs (r) do
        tmp = tmp .. " " .. k .. "='" .. v .. "'"
      end
      exec.sendport("*p", "ui", tmp)
    end
  end)

-------------------------------------------------------------------------------
--	Run application
-------------------------------------------------------------------------------

loadnetinterfaces(app)
-- ui.Application.connect(progwin)
app:addMember(netwin)
ui.Application.connect(netwin)
netwin:loadinterfaces()
app:addMember(filebox)
ui.Application.connect(filebox)

progwin:setValue("Status", "hide")
netwin:setValue("Status", "hide")
app:addMember(progwin)

app:run()

progtask:terminate()

app:hide()
app:cleanup()
