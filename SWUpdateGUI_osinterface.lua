--	(C) Copyright 2018
--	Stefano Babic, DENX Software Engineering, sbabic@denx.de.
--
--	SPDX-License-Identifier:     GPL-2.0-or-later

-------------------------------------------------------------------------------
--	Use standard tool to setup Network
-------------------------------------------------------------------------------

function ifup(setup)
  for _,intf in pairs(setup) do
    local cmd
    if  intf["name"] == "Gateway" then
      cmd = "route del default;route add default gw " .. intf["addr"]
    else
      if intf["dhcp"] then
        cmd = "udhcpc -n -i " .. intf["name"]
      else
        cmd = "ifconfig " .. intf["name"] .. " " .. intf["addr"] .. " netmask " .. intf["netmask"]
      end
    end
    os.execute(cmd)
  end
end
