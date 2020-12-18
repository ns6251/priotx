use ifstructs::ifreq;
use mio::{unix::SourceFd, Events, Interest, Poll, Token};
use nix::libc;
use std::{
    fs::{File, OpenOptions},
    io::{self, prelude::*},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    os::unix::io::{AsRawFd, RawFd},
};

const TUNSETIFF: libc::c_ulong = 0x400454CA;

#[derive(Debug)]
pub struct TunDevice {
    name: String,
    fd: File,
}

impl TunDevice {
    pub fn new(name: &str) -> io::Result<Self> {
        let mut ifr = ifreq::from_name(name)?;
        ifr.ifr_ifru.ifr_flags = (libc::IFF_TUN | libc::IFF_NO_PI) as _;

        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")?;

        match unsafe {
            libc::ioctl(
                fd.as_raw_fd(),
                TUNSETIFF,
                &mut ifr as *mut ifreq as *mut libc::c_void,
            )
        } {
            -1 => Err(std::io::Error::last_os_error()),
            _ => Ok(()),
        }?;

        let tun = Self {
            name: ifr.get_name()?,
            fd,
        };

        Ok(tun)
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

    pub fn get_rawfd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.fd.read(buf)
    }

    pub fn write(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.fd.write(buf)
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
pub struct Tunnel {
    tuns: [TunDevice; 2],
    addr: SocketAddr,
    dst: SocketAddr,
}

impl<'a> Tunnel {
    pub fn new(tuns: [TunDevice; 2], addr: SocketAddr, dst: SocketAddr) -> Self {
        Self { tuns, addr, dst }
    }

    pub fn tunnel(&mut self) -> io::Result<()> {
        let socket = UdpSocket::bind(self.addr)?;
        socket.set_nonblocking(true)?;

        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(3);

        for i in 0..2usize {
            poll.registry().register(
                &mut SourceFd(&self.tuns[i].get_rawfd()),
                Token(i),
                Interest::READABLE,
            )?;
        }
        poll.registry().register(
            &mut SourceFd(&socket.as_raw_fd()),
            Token(2),
            Interest::READABLE,
        )?;

        let mut buf = [0u8; 1500];

        loop {
            poll.poll(&mut events, None)?;
            for event in events.iter() {
                match event.token() {
                    Token(0) => {
                        let len = self.tuns[0].read(&mut buf)?;
                        let _send_len = socket.send_to(&mut buf[..len], &self.dst)?;
                        break;
                    }
                    Token(1) => {
                        let len = self.tuns[1].read(&mut buf)?;
                        let _send_len = socket.send_to(&mut buf[..len], &self.dst)?;
                    }
                    Token(2) => {
                        let (read_len, _) = socket.recv_from(&mut buf)?;
                        let _write_len = self.tuns[0].write(&mut buf[..read_len])?;
                    }
                    _ => unreachable!(),
                };
            }
        }
    }
}
