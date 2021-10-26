use echo_server::ServerAddr;
use echo_server::start_server;
use std::process;
use std::thread;
fn main() {
    let mut args:Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        args = vec![String::from(""), String::from("127.0.0.1"), String::from("23323")];
    }
    let addr = ServerAddr::new(&args).unwrap_or_else(|err| {
        println!("Error: {}", err);
        process::exit(1);
    });

    if let Err(e) = start_server(&addr) {
        println!("Error: {}", e);
    }
}