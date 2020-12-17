use ifstructs::ifreq;
use mio::{unix::SourceFd, Events, Interest, Poll, Token};
use nix::libc;
use pnet::packet::udp::MutableUdpPacket;
use std::{
    fs::{File, OpenOptions},
    io::{self, prelude::*},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    os::unix::io::{AsRawFd, RawFd},
    time::Duration,
    todo,
};

const TUNSETIFF: libc::c_ulong = 0x400454CA;

#[derive(Debug)]
pub struct TunDevice {
    name: String,
    fds: Vec<File>,
}

impl TunDevice {
    pub fn new(name: &str, queues: usize) -> io::Result<Self> {
        let mut tun = Self {
            name: String::new(),
            fds: Vec::with_capacity(queues),
        };

        tun.alloc_mq(name, queues)?;

        Ok(tun)
    }

    fn alloc_mq(&mut self, name: &str, queues: usize) -> io::Result<()> {
        let mut ifr = ifreq::from_name(name)?;
        ifr.ifr_ifru.ifr_flags = (libc::IFF_TUN | libc::IFF_NO_PI | libc::IFF_MULTI_QUEUE) as _;

        for _ in 0..queues {
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/net/tun")?;

            match unsafe {
                libc::ioctl(
                    f.as_raw_fd(),
                    TUNSETIFF,
                    &mut ifr as *mut ifreq as *mut libc::c_void,
                )
            } {
                -1 => Err(std::io::Error::last_os_error())?,
                _ => self.fds.push(f),
            };
        }

        self.name = ifr.get_name()?;

        Ok(())
    }

    pub fn up(&self) -> io::Result<()> {
        let mut ifr = ifreq::from_name(&self.name)?;
        if_ioctl(libc::SIOCGIFFLAGS, &mut ifr)?;

        ifr.set_flags(ifr.get_flags() | libc::IFF_UP as libc::c_short);

        if_ioctl(libc::SIOCSIFFLAGS, &mut ifr)
    }

    pub fn down(&self) -> io::Result<()> {
        let mut ifr = ifreq::from_name(&self.name)?;
        if_ioctl(libc::SIOCGIFFLAGS, &mut ifr)?;

        ifr.set_flags(ifr.get_flags() & !(libc::IFF_UP as libc::c_short));

        if_ioctl(libc::SIOCSIFFLAGS, &mut ifr)
    }

    pub fn get_addr(&self) -> io::Result<Ipv4Addr> {
        let mut ifr = ifreq::from_name(&self.name)?;
        if_ioctl(libc::SIOCGIFADDR, &mut ifr)?;

        unsafe {
            let addr: &libc::sockaddr_in = std::mem::transmute(&ifr.ifr_ifru.ifr_addr);
            Ok(std::mem::transmute(addr.sin_addr))
        }
    }

    pub fn set_addr(&self, addr: Ipv4Addr) -> io::Result<&Self> {
        let mut ifr = ifreq::from_name(&self.name)?;

        unsafe {
            let mut saddr_in: &mut libc::sockaddr_in =
                std::mem::transmute(&mut ifr.ifr_ifru.ifr_addr);

            saddr_in.sin_addr = std::mem::transmute(addr);
            saddr_in.sin_family = libc::AF_INET as _;
        }

        if_ioctl(libc::SIOCSIFADDR, &mut ifr)?;

        Ok(self)
    }

    pub fn get_dstaddr(&self) -> io::Result<Ipv4Addr> {
        let mut ifr = ifreq::from_name(&self.name)?;
        if_ioctl(libc::SIOCGIFDSTADDR, &mut ifr)?;

        unsafe {
            let addr: &libc::sockaddr_in = std::mem::transmute(&ifr.ifr_ifru.ifr_dstaddr);
            Ok(std::mem::transmute(addr.sin_addr))
        }
    }

    pub fn set_dstaddr(&self, dstaddr: Ipv4Addr) -> io::Result<&Self> {
        let mut ifr = ifreq::from_name(&self.name)?;

        unsafe {
            let mut saddr_in: &mut libc::sockaddr_in =
                std::mem::transmute(&mut ifr.ifr_ifru.ifr_dstaddr);

            saddr_in.sin_addr = std::mem::transmute(dstaddr);
            saddr_in.sin_family = libc::AF_INET as _;
        }

        if_ioctl(libc::SIOCSIFDSTADDR, &mut ifr)?;

        Ok(self)
    }

    pub fn get_netmask(&self) -> io::Result<Ipv4Addr> {
        let mut ifr = ifreq::from_name(&self.name)?;
        if_ioctl(libc::SIOCGIFNETMASK, &mut ifr)?;

        unsafe {
            let addr: &libc::sockaddr_in = std::mem::transmute(&ifr.ifr_ifru.ifr_netmask);
            Ok(std::mem::transmute(addr.sin_addr))
        }
    }

    pub fn set_netmask(&self, netmask: Ipv4Addr) -> io::Result<&Self> {
        let mut ifr = ifreq::from_name(&self.name)?;

        unsafe {
            let mut saddr_in: &mut libc::sockaddr_in =
                std::mem::transmute(&mut ifr.ifr_ifru.ifr_netmask);

            saddr_in.sin_addr = std::mem::transmute(netmask);
            saddr_in.sin_family = libc::AF_INET as _;
        }

        if_ioctl(libc::SIOCSIFNETMASK, &mut ifr)?;

        Ok(self)
    }

    pub fn get_rawfds(&self) -> Vec<RawFd> {
        self.fds.iter().map(AsRawFd::as_raw_fd).collect()
    }

    #[deprecated]
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(1024);

        for (i, fd) in self.fds.iter().enumerate() {
            poll.registry().register(
                &mut SourceFd(&fd.as_raw_fd()),
                Token(i),
                Interest::READABLE,
            )?;
        }
        // loop {
        for _ in 0..5 {
            poll.poll(&mut events, Some(Duration::new(5, 0)))?;
            for event in &events {
                let Token(n) = event.token();
                let len = self.fds[n].read(buf)?;
                let pk = MutableUdpPacket::new(buf);
                if let Some(pk) = pk {
                    println!("{:?}", pk);
                    println!("{} bytes at fd({})", len, self.fds[n].as_raw_fd());
                } else {
                    println!("hoge!");
                }
            }
        }
        Ok(0)
    }
}

fn if_ioctl(request: libc::c_ulong, ifr: &mut ifreq) -> io::Result<()> {
    let sock = UdpSocket::bind("0.0.0.0:55555")?;

    unsafe {
        match libc::ioctl(
            sock.as_raw_fd(),
            request,
            ifr as *mut ifreq as *mut libc::c_void,
        ) {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        }
    }
}

#[derive(Debug)]
struct Tunnel<'a> {
    tuns: Vec<&'a TunDevice>,
    addr: SocketAddr,
    dst: SocketAddr,
}

impl<'a> Tunnel<'a> {
    pub fn new(tuns: &[&'a TunDevice], addr: SocketAddr, dst: SocketAddr) -> Self {
        Self {
            tuns: Vec::from(tuns),
            addr,
            dst,
        }
    }

    pub fn tunnel(&self) -> io::Result<()> {
        let socket = UdpSocket::bind(self.addr)?;
        // socket.set_nonblocking(true)?;

        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(1024);

        todo!()
    }
}
