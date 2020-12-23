use std::net::UdpSocket;
use std::str;
use std::{env, io};

pub fn communicate(addr: &str) -> anyhow::Result<()> {
    // let sock = UdpSocket::bind("192.0.2.1:0")?;
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        sock.send_to(input.as_bytes(), addr)?;

        let mut buff = [0u8; 1024];
        sock.recv_from(&mut buff).expect("faild to receive");
        println!(
            "{}",
            str::from_utf8(&buff).expect("failed to convert to String")
        );
    }
}

fn main() -> anyhow::Result<()> {
    let args = env::args().collect::<Vec<String>>();
    let addr = args[1].as_str();
    communicate(addr)?;

    Ok(())
}
