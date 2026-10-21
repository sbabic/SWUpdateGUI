#!/bin/sh

#
# SPDX-License-Identifier: GPL-2.0-or-later
#
# Copyright (c) 2024 IFM Ecomatic GmbH
#


set -e

interface=""
ip_address=""
gateway="0.0.0.0"

print_help()
{
	echo "Description:"
	echo "    Reconfigures a network interface using a static configuration."
	echo
	echo "Usage:"
	echo "    $(basename "$0") [OPTION]"
	echo
	echo "Options:"
	echo "    --interface           - interface name"
	echo "    --ip                  - static ip address in the IPv4 CIDR format (aaa.aaa.aaa.aaa/nnn)"
	echo "    --gateway <optional>  - gateway IP address, if this option is omitted then"
	echo "                            the default gateway will be 0.0.0.0"
	echo
	echo "Return values:"
	echo "    0 when the operation succeed"
	echo "    1 when the operation fails"
	echo "    2 for invalid options passed to the script"
	echo
	echo "WARNING: If it fails to set the ip address, the network interface will not have"
	echo "         an ip address assigned anymore."
	echo

	exit 2
}

verify_if_required_parameters_are_set()
{
	if [ -z "$interface" ] || [ -z "$ip_address" ]
	then
		echo "The parameters --interface and --ip are not optional."
		echo "Both the network interface and ip address have to be set."
		echo

		print_help
	fi
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
			--ip)
				ip_address=$value
				;;
			--gateway)
				gateway=$value
				;;
			*)
				echo "Unknown option $key"
				echo
				print_help
				;;
		esac

		shift;
	done

	verify_if_required_parameters_are_set
}

set_ip_address_failed()
{
	echo "Failed to set the ip address."
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
	# a running dhcp client would replace the static address at renewal
	stop_dhcp_client
	ip -4 address flush label "$interface" || set_ip_address_failed
	ip addr add "$ip_address" dev "$interface" || set_ip_address_failed

	ip route replace default via "$gateway" dev "$interface" || set_ip_address_failed
}


# main program starts here

parse_and_verify_commandline_options "$@"

reconfigure_network_interface
