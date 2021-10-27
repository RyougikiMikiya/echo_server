extern crate mio;
use mio::event::{Event, Source};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Registry, Token};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::str::from_utf8;
use crate::ServerAddr;
const SERVER: Token = Token(0);

pub fn start_poll_server(addr : &ServerAddr) -> io::Result<()>{
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(128);
    let socket = addr.to_socket_addr();
    let mut server = TcpListener::bind(socket)?;
    poll.registry().register(&mut server, SERVER, Interest::READABLE)?;
    let mut connections = HashMap::new();
    let mut unique_token = Token(SERVER.0 + 1);

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    let (mut connection, address) = match server.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    };
                    println!("Accepted connection from: {}", address);
                    let token = next(&mut unique_token);
                    poll.registry().register(&mut connection, token, Interest::READABLE)?;
                    
                    connections.insert(token, connection);
                },
                token => {
                    let done = if let Some(connection) = connections.get_mut(&token) {
                        handle_connection_event(poll.registry(), connection, event)?
                    } else {
                        unreachable!("token should {} in the map!", token.0);
                    };
                    if done {
                        // let connection = connections.get_mut(&token).unwrap();
                        // poll.registry().deregister(connection)?;
                        connections.remove(&token);
                    }
                } 
            }
        }
    }
}

fn next(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
}

fn handle_connection_event(
    registry: &Registry,
    connection: &mut TcpStream,
    event: &Event,
) -> io::Result<bool> {
    println!("is_readable {} is_writable {}", event.is_readable(), event.is_writable());
    if event.is_readable() {
        let mut connection_closed = false;
        let mut received_data = vec![0; 4096];
        let mut bytes_read = 0;
        loop {
            match connection.read(&mut received_data[bytes_read..]) {
                Ok(0) => {
                    connection_closed = true;
                    break;
                }
                Ok(n) => {
                    bytes_read += n;
                    if bytes_read == received_data.len() {
                        received_data.resize( bytes_read + 4096, 0);
                    }
                }
                //这里底层默认是非阻塞套接字的，所以就是靠这个特性来跳出循环,标准库的TcpStream需要手动调用set_nonblocking
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }

        if bytes_read != 0 {
            let received_data = &received_data[..bytes_read];
            if let Ok(str_buf) = from_utf8(received_data) {
                println!("<: {} in {} bytes", str_buf, bytes_read);
            } else {

            }
            // registry.reregister(connection, event.token(), Interest::WRITABLE)?;
            match connection.write(&received_data[..bytes_read]){
                Ok(n) if n != bytes_read => println!("should write {} actual {} {:#?}", bytes_read, n, io::ErrorKind::WriteZero),
                Ok(byte_write) => {
                    println!(">: {} in {} bytes", String::from_utf8_lossy(&received_data[..byte_write]), byte_write);
                    registry.reregister(connection, event.token(), Interest::READABLE)?
                }
                Err(ref err) if would_block(err) => {
                    println!("{}",err);
                }
                Err(ref err)if interrupted(err) => {
                    println!("{}",err);
                }
                Err(err) => println!("{}", err)
            }
        }

        // if event.is_writable() {
        //     match connection.write(&received_data[..bytes_read]){
        //         Ok(n) if n != bytes_read => println!("should write {} actual {} {:#?}", bytes_read, n, io::ErrorKind::WriteZero),
        //         Ok(byte_write) => {
        //             println!(">: {} in {} bytes", from_utf8(&received_data[..byte_write]).unwrap(), byte_write);
        //             registry.reregister(connection, event.token(), Interest::READABLE)?
        //         }
        //         Err(ref err) if would_block(err) => {
        //             println!("{}",err);
        //         }
        //         Err(ref err)if interrupted(err) => {
        //             println!("{}",err);
        //         }
        //         Err(err) => println!("{}", err)
        //     }
        // }
        
        if connection_closed {
            println!("Connection closed");
            return Ok(true);
        }
    }
    Ok(false)
}

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}