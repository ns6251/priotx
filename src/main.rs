use priotx::{TunDevice, Tunnel};
use std::net::Ipv4Addr;

fn main() -> anyhow::Result<()> {
    let tun = TunDevice::new("")?;

    tun.set_addr(Ipv4Addr::new(10, 60, 0, 1))?
        .set_dstaddr(Ipv4Addr::new(10, 60, 1, 2))?
        .set_netmask(Ipv4Addr::new(255, 255, 255, 254))?
        .up()?;

    println!(
        "addr: {:?}, dstaddr: {:?}, netmask: {:?}",
        tun.get_addr()?,
        tun.get_dstaddr()?,
        tun.get_netmask()?
    );

    let tun2 = TunDevice::new("")?;

    tun2.set_addr(Ipv4Addr::new(10, 60, 1, 1))?
        .set_dstaddr(Ipv4Addr::new(10, 60, 0, 2))?
        .set_netmask(Ipv4Addr::new(255, 255, 255, 254))?
        .up()?;

    println!(
        "addr: {:?}, dstaddr: {:?}, netmask: {:?}",
        tun2.get_addr()?,
        tun2.get_dstaddr()?,
        tun2.get_netmask()?
    );

    let mut tunnel = Tunnel::new(
        [tun, tun2],
        "127.0.0.1:12345".parse()?,
        "127.0.0.1:23456".parse()?,
    );

    tunnel.tunnel()?;

    Ok(())
}
