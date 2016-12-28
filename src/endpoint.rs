use rori_utils::data::RoriData;
use rori_utils::client::{RoriClient, ConfigServer};
use rustc_serialize::json::decode;
use std::path::Path;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::str::from_utf8;
use std::io::prelude::*;
use std::fs::File;

#[allow(dead_code)]
struct Client {
    stream: TcpStream,
}

#[allow(dead_code)]
impl Client {
    fn new(stream: TcpStream) -> Client {
        Client { stream: stream }
    }

    fn read(&mut self) -> String {
        let mut result = String::from("");
        let mut buffer = [0u8; 512];
        loop {
            let usize = self.stream.read(&mut buffer).unwrap();
            if usize == 0 {
                break;
            }
            let msg = from_utf8(&buffer).unwrap();
            result.push_str(msg);
        }
        result
    }
}

#[derive(Clone, RustcDecodable, RustcEncodable, Default, PartialEq, Debug)]
struct RoriServer {
    rori_ip: Option<String>,
    rori_port: Option<String>,
}

#[derive(Clone, RustcDecodable, RustcEncodable, Default, PartialEq, Debug)]
struct EndpointDetails {
    owner: Option<String>,
    name: Option<String>,
    compatible_types: Option<String>,
}

#[allow(dead_code)]
pub struct Endpoint {
    address: String,
    rori_address: String,
    pub is_registered: bool,
    owner: String,
    name: String,
    compatible_types: String,
}

#[allow(dead_code)]
impl Endpoint {
    fn parse_config_server(data: String) -> String {
        let params: ConfigServer = decode(&data[..]).unwrap();
        format!("{}:{}",
                &params.ip.unwrap_or(String::from("")),
                &params.port.unwrap_or(String::from("")))
    }

    fn parse_config_rori(data: String) -> String {
        let params: RoriServer = decode(&data[..]).unwrap();
        format!("{}:{}",
                &params.rori_ip.unwrap_or(String::from("")),
                &params.rori_port.unwrap_or(String::from("")))
    }

    pub fn new<P: AsRef<Path>>(config: P) -> Endpoint {
        // Configure from file
        let mut file = File::open(config)
            .ok()
            .expect("Config file not found");
        let mut data = String::new();
        file.read_to_string(&mut data)
            .ok()
            .expect("failed to read!");
        let address = Endpoint::parse_config_server(data.clone());
        let rori_address = Endpoint::parse_config_rori(data.clone());
        let details: EndpointDetails = decode(&data[..]).unwrap();
        if address == ":" || rori_address == ":" {
            error!(target:"endpoint", "Empty config for the connection to the server");
        }
        Endpoint {
            address: address,
            rori_address: rori_address,
            is_registered: false,
            owner: details.owner.unwrap_or(String::from("")),
            name: details.name.unwrap_or(String::from("")),
            compatible_types: details.compatible_types.unwrap_or(String::from("")),
        }
    }

    pub fn start(&self, vec: Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind(&*self.address).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut client = Client::new(stream.try_clone().unwrap());
                    let content = client.read();
                    info!(target:"endpoint", "Received:{}", &content);
                    let end = content.find(0u8 as char);
                    let (content, _) = content.split_at(end.unwrap_or(content.len()));
                    let data_to_process = RoriData::from_json(String::from(content));
                    if data_to_process.datatype == "text" {
                        vec.lock().unwrap().push(data_to_process.content);
                    }
                }
                Err(e) => {
                    error!(target:"endpoint", "{}", e);
                }
            };
        }
        drop(listener);
    }

    pub fn register(&mut self) {
        info!(target:"endpoint", "try to register endpoint");
        // TODO security and if correctly registered
        let rori_address = self.rori_address.clone();
        let address = self.address.clone();
        let mut client = RoriClient { address: rori_address };
        let mut content = String::from(address);
        content.push_str("|");
        content.push_str(&*self.compatible_types);
        self.is_registered = client.send_to_rori(&self.owner, &*content, &self.name, "register");
    }
}
