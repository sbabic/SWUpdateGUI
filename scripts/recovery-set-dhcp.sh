#!/bin/sh

#
# SPDX-License-Identifier: GPL-2.0-or-later
#
# Copyright (c) 2024 IFM Ecomatic GmbH
#


set -e

interface=""

print_help()
{
	echo "Description:"
	echo "    Reconfigures a network interface as a DHCP client."
	echo
	echo "Usage:"
	echo "    $(basename "$0") [OPTION]"
	echo
	echo "Options:"
	echo "    --interface   - interface name"
	echo
	echo "Return values:"
	echo "    0 when the operation succeed"
	echo "    1 when the operation fails"
	echo "    2 for invalid options passed to the script"
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
			echo "$key empty value"
			echo
			print_help
		fi

		case $key in
			--interface)
				interface=$value
				;;
			*)
				echo "Unknown option $key"
				echo
				print_help
				;;
		esac

		shift;
	done
}

failed_to_reconfigure_as_dhcp_client()
{
	echo "Failed to reconfigure the network interface ($interface) as a dhcp client."

	exit 1
}

stop_dhcp_client()
{
	for pid in $(pgrep -f "udhcpc .*-i $interface\$")
	do
		kill "$pid" || true
	done
}

reconfigure_network_interface()
{
	# only one dhcp client per interface
	stop_dhcp_client
	udhcpc -n -i "$interface" || failed_to_reconfigure_as_dhcp_client
}


# main program starts here

parse_and_verify_commandline_options "$@"

reconfigure_network_interface
