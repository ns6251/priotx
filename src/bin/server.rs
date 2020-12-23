use std::{env, net::UdpSocket, str};

pub fn serve(addr: &str) -> anyhow::Result<()> {
    let sock = UdpSocket::bind(addr)?;
    loop {
        let mut buf = [0u8; 1024];
        let (size, src) = sock.recv_from(&mut buf)?;
        println!("Handling data from {}", src);
        println!("{}", str::from_utf8(&buf[..size])?);
        sock.send_to(&buf, src)?;
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let addr = args[1].as_str();
    serve(addr)?;

    Ok(())
}
