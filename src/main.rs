use priotx::{TunDevice, Tunnel};
use std::env;

fn main() -> anyhow::Result<()> {
    let args = env::args().map(String::from).collect::<Vec<_>>();
    anyhow::ensure!(
        args.len() == 6,
        "requirement args: <ADDR> <DSTADDR> <TUN1ADDR> <TUN2ADDR> <TUNDSTADDR>"
    );

    let addr = args[1].parse().expect("<ADDR> parse failed");
    let dst = args[2].parse().expect("<DSTADDR> parse failed");
    let tun1addr = args[3].parse().expect("<TUN1ADDR> parse failed");
    let tun2addr = args[4].parse().expect("<TUN2ADDR> parse failed");
    let tundst = args[5].parse().expect("<TUNDSTADDR> parse failed");

    let tun1 = TunDevice::new("")?;
    tun1.set_addr(tun1addr)?
        .set_dstaddr(tundst)?
        .set_netmask("255.255.255.254".parse()?)?
        .up()?;

    println!(
        "addr: {:?}, dstaddr: {:?}, netmask: {:?}",
        tun1.get_addr()?,
        tun1.get_dstaddr()?,
        tun1.get_netmask()?
    );

    let tun2 = TunDevice::new("")?;
    tun2.set_addr(tun2addr)?
        .set_dstaddr(tundst)?
        .set_netmask("255.255.255.254".parse()?)?
        .up()?;

    println!(
        "addr: {:?}, dstaddr: {:?}, netmask: {:?}",
        tun2.get_addr()?,
        tun2.get_dstaddr()?,
        tun2.get_netmask()?
    );

    let mut tunnel = Tunnel::new([tun1, tun2], addr, dst);
    tunnel.tunnel()?;

    Ok(())
}
