use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use micro_types::tx::primitives::PackedEthSignature;
use micro_types::H256;

const PING_INTERVAL: u64 = 3; // Ping interval time (seconds)
const TIMEOUT: u64 = 3; // Ping timeout time (seconds)
const MAX_RETRIES: usize = 5; // Maximum number of retries

pub fn run(mut server_address: &str, private_key: H256) {
    vlog::info!(
        "Started the external check heartbeat server_address:{}",
        server_address
    );

    if server_address.is_empty() {
        server_address = "127.0.0.1:3052";
    }
    let mut retries = 0; //127.0.0.1:8080
    loop {
        match connect_and_ping(server_address, &private_key, retries) {
            Ok(_) => {
                vlog::debug!("Ping successful retries: {}", retries);
                retries = 0;
            }
            Err(err) => {
                retries += 1;
                thread::sleep(Duration::from_secs(PING_INTERVAL * retries as u64));
                if retries > MAX_RETRIES {
                    vlog::error!("Maximum retry limit reached. Exiting. err:{}", err);
                    break;
                }
            }
        }
        thread::sleep(Duration::from_secs(PING_INTERVAL));
    }
}

fn connect_and_ping(
    server_address: &str,
    private_key: &H256,
    retries: usize,
) -> Result<(), String> {
    let mut stream = match TcpStream::connect(server_address) {
        Ok(stream) => stream,
        Err(_) => {
            if retries < MAX_RETRIES {
                return Err("Failed to receive PONG".to_string());
            } else {
                return Err("Failed to connect  and reached maximum retry limit".to_string());
            }
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(TIMEOUT)))
        .unwrap();
    let message = b"ping";
    match PackedEthSignature::sign(private_key, message).ok() {
        Some(val) => {
            stream.write_all(&val.serialize_packed()).unwrap();
            vlog::info!("connect_and_ping stream: {:?}", stream);
            stream.shutdown(std::net::Shutdown::Both).unwrap();
            return Ok(());
        }
        None => {
            return Err("Failed to PackedEthSignature".to_string());
        }
    };
}
