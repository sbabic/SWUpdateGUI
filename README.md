<p align ="center"><img src=logo.png width=400 height=400 /></p>

RescueGUI - a simple GUI for SWUpdate in rescue mode
====================================================

RescueGUI is a simple GUI written in Lua with the help of the tekui
graphic toolkit (http:http://tekui.neoscientists.org/index.html).
The goal is to have a GUI for HMI devices when SWUpdate is started
in rescue mode. 

The GUI has the following goal:

- GUI must show when an update is started from one of the interface
  and displays the progress and status.
- It can initiate an install from a local storage as SD / USB
- It can provide stup for SWUpdate like network interface, etc.
- Footprint should be as small as possible to be merged into the rescue image.
- Build should be integrated in meta-swupdate

Requirements
------------

- Lua interpreter > 5.2 (previous versions untested)
- tekui and luafilesystem module
- SWUpdate binding (lua-swupdate)

Starting
--------

SWUpdateGUI.lua is the main program.

Configuration
-------------

The GUI tries to load a config.lua file at the startup. See comments in this
file to check what can be configured.

License
-------
This project is licensed under GPL version 2.0+.

