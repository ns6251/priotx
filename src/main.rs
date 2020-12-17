use priotx::TunDevice;
use std::net::Ipv4Addr;

fn main() -> anyhow::Result<()> {
    let mut tun = TunDevice::new("", 4)?;
    // println!("{:#?}", tun);

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

    let mut tun2 = TunDevice::new("", 4)?;
    // println!("{:#?}", tun2);

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

    let mut buf = String::new();
    println!("enter to continue");
    std::io::stdin().read_line(&mut buf)?;

    let socket = std::net::UdpSocket::bind("10.60.0.1:33333")?;
    socket.send_to(b"HELLO", "10.60.1.2:33334")?; // reach packet to tun0

    let mut buf = [0u8; 1500];
    println!("{:?}", tun.read(&mut buf)?);

    Ok(())
}
