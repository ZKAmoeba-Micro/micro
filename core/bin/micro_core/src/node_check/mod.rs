use lazy_static::lazy_static;
use micro_types::tx::primitives::PackedEthSignature;
use micro_types::Address;
use std::io::{ErrorKind::WouldBlock, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::RwLock;
use std::thread;
// Create a struct to hold the addresses
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::watch;
const PING_INTERVAL: u64 = 5; // Ping interval time (seconds)

lazy_static! {
    static ref GLOBAL_SET: RwLock<HashMap<Address, Instant>> = RwLock::new(HashMap::new());
}

fn add_address(address: Address, time: Instant) {
    let mut map = GLOBAL_SET.write().unwrap();
    map.insert(address, time);
}

pub fn get_all_addresses() -> Vec<Address> {
    let global_set = GLOBAL_SET.read().unwrap();
    global_set.keys().cloned().collect()
}

fn delete_heartbeat_info_before_time(new_time: Instant) {
    let mut map = GLOBAL_SET.write().unwrap();
    map.retain(|_, time| sub(new_time, *time) <= PING_INTERVAL);
}

fn sub(new_time: Instant, time: Instant) -> u64 {
    let t = new_time.duration_since(time).as_secs();
    t
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckNode {
    server_address: String,
}

impl CheckNode {
    pub fn new(server_address: String) -> Self {
        let mut check_node = Self { server_address };
        if check_node.server_address.is_empty() {
            check_node.server_address = String::from("0.0.0.0:3052");
        }
        check_node
    }

    fn handle_client(&self, mut stream: TcpStream) {
        let mut buffer: Vec<u8> = Vec::new();
        while let Ok(size) = stream.read_to_end(&mut buffer) {
            if size == 0 {
                break;
            }
            let msg = String::from_utf8_lossy(&buffer[..size]);
            match PackedEthSignature::deserialize_packed(&buffer[..size]) {
                Ok(val) => {
                    let message = b"ping";
                    let msg_bytes = PackedEthSignature::message_to_signed_bytes(message);
                    match val.signature_recover_signer(&msg_bytes) {
                        Ok(addr) => {
                            add_address(addr, Instant::now());
                            if msg.starts_with("ping") {
                                let response = "pong".as_bytes();
                                stream.write(response).unwrap();
                            }
                        }
                        Err(_) => {
                            vlog::error!("nodeCheck error signature_recover_signer msg:{}", msg);
                            stream.shutdown(std::net::Shutdown::Both).unwrap();
                        }
                    }
                }
                Err(_) => {
                    vlog::error!("nodeCheck error deserialize_packed msg:{}", msg);
                    stream.shutdown(std::net::Shutdown::Both).unwrap();
                }
            }
        }
    }

    pub async fn run(self, stop_receiver: watch::Receiver<bool>) {
        vlog::info!("Started nodeCheck server_address:{}", self.server_address);
        let listener = TcpListener::bind(self.server_address.clone());
        let stop_receiver_clone = stop_receiver.clone();
        match listener {
            Ok(tcp) => {
                tcp.set_nonblocking(true)
                    .expect("nodeCheck Failed to set non-blocking mode");
                for stream in tcp.incoming() {
                    if *stop_receiver_clone.borrow() {
                        vlog::info!("Stop nodeCheck run is shutting down");
                        drop(tcp);
                        return;
                    }
                    match stream {
                        Ok(stream) => {
                            self.handle_client(stream);
                        }
                        Err(ref e) if e.kind() == WouldBlock => {
                            continue;
                        }
                        Err(e) => {
                            vlog::error!("Connection stream failed: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                vlog::error!(
                    "Connection failed server_address: {},e:{}",
                    self.server_address,
                    e
                );
                thread::sleep(Duration::from_secs(PING_INTERVAL));
            }
        }
    }

    pub async fn delete_heartbeat_address(self, stop_receiver: watch::Receiver<bool>) {
        loop {
            if *stop_receiver.borrow() {
                vlog::info!("Stop signal received, nodeCheck is shutting down");
                break;
            }
            thread::sleep(Duration::from_secs(PING_INTERVAL + 1));
            let start = Instant::now();
            if !get_all_addresses().is_empty() {
                delete_heartbeat_info_before_time(start);
            }
        }
    }
}
