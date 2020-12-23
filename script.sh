#!/usr/bin/bash -eu

readonly bin="./target/debug/priotx"

# network namespace name
readonly ns1="ns1"
readonly ns2="ns2"
readonly router="router"

# veth name
readonly ns1_veth0="${ns1}-veth0"
readonly ns2_veth0="${ns2}-veth0"
readonly router_veth0="gw-veth0"
readonly router_veth1="gw-veth1"

# addresses
readonly seg1=(192 0 2 0 24)
readonly seg2=(198 51 100 0 24)
readonly ns1_addr="${seg1[0]}.${seg1[1]}.${seg1[2]}.1"
readonly ns2_addr="${seg2[0]}.${seg2[1]}.${seg2[2]}.1"
readonly rt_seg1_addr="${seg1[0]}.${seg1[1]}.${seg1[2]}.254"
readonly rt_seg2_addr="${seg2[0]}.${seg2[1]}.${seg2[2]}.254"

function close_all_netns() {
    for ns in $(ip netns list | awk '{print $1}'); do
        ip netns delete $ns
    done
}

function setup() {
    # network namespaceの作成
    ip netns add ${ns1}
    ip netns add ${ns2}
    ip netns add ${router}

    # network namespace同士を接続するvethインターフェースの作成
    ip link add ${ns1_veth0} type veth peer name ${router_veth0}
    ip link add ${ns2_veth0} type veth peer name ${router_veth1}

    # 作成したvethインターフェースをnetwork namespaceに所属させる
    ip link set ${ns1_veth0} netns ${ns1}
    ip link set ${ns2_veth0} netns ${ns2}
    ip link set ${router_veth0} netns ${router}
    ip link set ${router_veth1} netns ${router}

    # link up
    ip netns exec ${ns1} ip link set ${ns1_veth0} up
    ip netns exec ${ns2} ip link set ${ns2_veth0} up
    ip netns exec ${router} ip link set ${router_veth0} up
    ip netns exec ${router} ip link set ${router_veth1} up

    # add ip address
    ip netns exec ${ns1} ip address add "${ns1_addr}/24" dev ${ns1_veth0}
    ip netns exec ${router} ip address add "${rt_seg1_addr}/24" dev ${router_veth0}
    ip netns exec ${ns2} ip address add "${ns2_addr}/24" dev ${ns2_veth0}
    ip netns exec ${router} ip address add "${rt_seg2_addr}/24" dev ${router_veth1}

    # routing
    ip netns exec ${ns1} ip route add default via ${rt_seg1_addr}
    ip netns exec ${ns2} ip route add default via ${rt_seg2_addr}
    ip netns exec ${ns1} sysctl net.ipv4.ip_forward=1
    ip netns exec ${ns2} sysctl net.ipv4.ip_forward=1
    ip netns exec ${router} sysctl net.ipv4.ip_forward=1
}

function host1() {
    # <ADDR> <DSTADDR> <TUN1ADDR> <TUN2ADDR> <TUNDSTADDR>
    ip netns exec ${ns1} ${bin} "192.0.2.1:33333" "198.51.100.1:44444" "10.60.0.1" "10.60.1.1" "10.61.0.1"
}

function host2() {
    ip netns exec ${ns2} ${bin} "198.51.100.1:44444" "192.0.2.1:33333" "10.61.0.1" "10.61.1.1" "10.60.0.1"
}

function client() {
    ip netns exec ${ns1} "./target/debug/client" "10.61.0.1:23456"
}

function server() {
    ip netns exec ${ns2} "./target/debug/server" "10.61.0.1:23456"
}

case "${1}" in
"host1") host1 ;;
"host2") host2 ;;
"setup") setup ;;
"close") close_all_netns ;;
"client") client ;;
"server") server ;;
*) echo "arg: { setup | close | host1 | host2 | client | server }" ;;
esac
