#!/bin/sh

#
# SPDX-License-Identifier: GPL-2.0-or-later
#
# Copyright (c) 2024 IFM Ecomatic GmbH
#


address=""

print_help()
{
	echo "Description:"
	echo "    This script tells whether given address is configured via DHCP or not"
	echo
	echo "Usage:"
	echo "    $(basename "$0") [OPTION]"
	echo
	echo "Options:"
	echo "    --address  - IP Address to check whether it was configured as a DHCP client"
	echo
	echo "Output:"
	echo "    static: when address is configured as static IP address"
	echo "    dhcp:   when address is configured as a DHCP client"
	echo
	echo "Return values:"
	echo "    0 when address is configured as a DHCP client"
	echo "    1 when the ip address configured as a static IP address"
	echo "    2 invalid argument passed to the script"
	echo

	exit 2
}

parse_and_verify_commandline_options()
{
	if [ $# -eq 0 ]
	then
		print_help
	fi

	while [ $# -gt 0 ]
	do
		key=${1%=*}
		value=${1#"$key"=}
		if [ -z "$value" ]
		then
			echo "Value must be provided for $key"
			print_help
		fi

		case $key in
			--address)
				address=$value
				;;
			*)
				echo "Unknown option $key"
				print_help
				;;
		esac
		
		shift;
	done
}

ip_addres_is_not_found()
{
	echo "the ip address '$address' is not found"

	exit 2
}

ip_addres_is_of_a_dhcp_client()
{
	echo "dhcp"

	exit 0
}

ip_addres_is_staticaly_configured()
{
	echo "static"

	exit 1
}


# main program starts here

parse_and_verify_commandline_options "$@"

if [ -z "$address" ]
then
	echo "address must be provided"
	print_help
fi

# check if the ip address is found, the trailing "/" avoids that
# 192.168.1.1 matches 192.168.1.10
cmd_output=$(ip -o -4 address show | grep " inet $address/") || ip_addres_is_not_found

# An address with a lease time (systemd-networkd) is flagged as "dynamic".
# udhcpc sets the address without lease time: check whether it runs for
# the interface.
interface=$(echo "$cmd_output" | awk '{ print $2; exit }')

if ( echo "$cmd_output" | grep -q " dynamic " ) ||
   pgrep -f "udhcpc .*-i $interface\$" > /dev/null
then
	ip_addres_is_of_a_dhcp_client
else
	ip_addres_is_staticaly_configured
fi

