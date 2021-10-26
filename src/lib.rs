#[cfg(test)]
#[macro_use]
extern crate assert_matches;

use std::{
    error::Error,
    io::{Read, Write},
    net::*,
    str::FromStr,
};

pub fn start_server(addr: &ServerAddr) -> Result<(), Box<dyn Error>> {
    let pool = thread_pool::ThreadPool::new(4);

    let socket = SocketAddr::new(IpAddr::V4(addr.addr), addr.port);
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
    addr: Ipv4Addr,
    port: u16,
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
}

pub mod thread_pool {
    use std::thread;
    use std::sync::mpsc;
    use std::sync::Mutex;
    use std::sync::Arc;

    pub struct ThreadPool{
        workers :Vec<Worker>,
        sender: mpsc::Sender<Job>,
    }

    //fnonce() 不关心返回值
    type Job = Box<dyn FnOnce() + Send + 'static>;

    struct Worker {
        id: usize,
        thread: thread::JoinHandle<()>,
    }

    impl Worker {
        fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
            let thread =  thread::spawn( move ||{
                loop {
                    // let 语句结束后，临时变量的lockguard就被丢掉了，所以也释放了锁，这才能让下一个线程可以继续收到消息再取走任务
                    let job = receiver.lock().unwrap().recv().unwrap();
                    job();
                    // 显示的例子，这样guard的生命期覆盖了job，job中又有read阻塞住了,job不执行完那mutex就不会释放，其他线程也无法拿到receiver的job，就必须显示的drop掉guard来释放锁。
                    // let guard = receiver.lock().unwrap();
                    // let job = guard.recv().unwrap();
                    // std::mem::drop(guard);
                    // job();
                }
                // 更傻逼了，while let是表达式，临时变量gurad在整个表达式期间都有效所以没法释放
                // while let Ok(job) = receiver.lock().unwrap().recv() {
                //     println!("Worker {} got a job; executing.", id);
                //     job();
                // }
            });
            Worker { id: id, thread: thread}
        }
    }

    impl ThreadPool {
        pub fn new(size: usize) -> ThreadPool {
            assert!(size > 0);
            let (tx, rx) = mpsc::channel();
            let receiver = Arc::new(Mutex::new(rx));
            let mut workers = Vec::with_capacity(size);
            for id in 0..size {
                workers.push(Worker::new(id, receiver.clone()));
            }

            ThreadPool {
                workers,
                sender: tx,
            }
        }

        pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static,
        {
            let job = Box::new(f);
            self.sender.send(job).unwrap();
        }
    }
}
