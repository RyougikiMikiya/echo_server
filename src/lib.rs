#[cfg(test)]
#[macro_use]
extern crate assert_matches;
use std::{
    error::Error,
    io::{Read, Write},
    net::*,
    str::FromStr,
};
mod thread_pool;

pub fn start_server(addr: &ServerAddr) -> Result<(), Box<dyn Error>> {
    let pool = thread_pool::ThreadPool::new(4);

    let socket = addr.to_socket_addr();
    let listener = TcpListener::bind(socket)?;
    println!("echo server is running on {}:{}", addr.addr, addr.port);
    for stream in listener.incoming() {
        let stream = stream?;
        pool.execute(||{
            if let Err(e) = handle_echo_stream(stream){
                println!("Error: {}", e);
            }
        })
    }
    Ok(())
}

fn handle_echo_stream(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let client_addr = stream.peer_addr()?;
    println!(
        "client from {} {} has connected",
        client_addr.ip(),
        client_addr.port()
    );
    let mut buffer = [0; 1024];
    let rbytes = stream.read(&mut buffer)?;
    if rbytes == 0 {
        println!("peek close connection");
        return Ok(());
    }
    println!("> {} in {} bytes", String::from_utf8_lossy(&buffer), rbytes);
    let wbytes = stream.write(&buffer[..rbytes])?;
    if wbytes != 0 {
        assert_eq!(wbytes, rbytes);
    }
    println!(
        "< {} in {} bytes",
        String::from_utf8_lossy(&buffer[..rbytes]),
        wbytes
    );

    Ok(())
}

#[derive(Debug)]
pub struct ServerAddr {
    pub addr: Ipv4Addr,
    pub port: u16,
}

impl ServerAddr {
    pub fn new(args: &[String]) -> Result<Self, String> {
        if args.len() != 3 {
            return Err(String::from("not enough arguments"));
        }
        let addr: Ipv4Addr;
        match Ipv4Addr::from_str(&args[1]) {
            Ok(ipv4_addr) => addr = ipv4_addr,
            Err(err) => {
                let s = format!("parse addr failed due to {}", err);
                return Err(s);
            }
        }
        let port: u16;
        match args[2].parse::<u16>() {
            Ok(p) => port = p,
            Err(err) => {
                let s = format!("parse port failed due to {}", err);
                return Err(s);
            }
        }
        Ok(ServerAddr {
            addr: addr,
            port: port,
        })
    }
    pub fn to_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(self.addr), self.port)
    }
}

#[derive(Debug)]
enum Foo {
    A(i32),
    B(i32),
}
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let a = Foo::A(1);

        assert_matches!(a, Foo::A(_));

        assert_matches!(a, Foo::A(i) if i == 1);
    }

    #[test]
    fn test_parse_args() {
        let args1 = vec![
            String::from(""),
            String::from("127.0.0.1"),
            String::from("5534"),
        ];
        assert_matches!(ServerAddr::new(&args1), Ok(_));
        assert_eq!(
            args1[1].parse::<Ipv4Addr>().unwrap(),
            ServerAddr::new(&args1).unwrap().addr
        );
        assert_eq!(
            args1[2].parse::<u16>().unwrap(),
            ServerAddr::new(&args1).unwrap().port
        );

        let args2 = vec![
            String::from(""),
            String::from("554.0.0.1"),
            String::from("5534"),
        ];
        assert_matches!(ServerAddr::new(&args2), Err(_));

        let args3 = vec![
            String::from(""),
            String::from("10.7.0.189"),
            String::from("75534"),
        ];
        assert_matches!(ServerAddr::new(&args3), Err(_));
    }

    #[test]
    fn test_to_socket_addr() {

    }
}