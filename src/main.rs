extern crate irc;
extern crate rustc_serialize;
extern crate regex;

mod rori_utils;

use irc::client::prelude::*;
use std::io::prelude::*;
use std::path::Path;
use std::net::{TcpListener, TcpStream};
use std::str::from_utf8;
use rori_utils::client::{ConfigServer, RoriClient};
use rori_utils::data::RoriData;
use std::thread;
use std::sync::{Arc, Mutex};
use rustc_serialize::json::decode;
use std::fs::File;

struct Client {
    stream: TcpStream,
}

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

struct RoriIrcEntry {
    server: IrcServer,
}

impl RoriIrcEntry {
    fn new<P: AsRef<Path>>(config: P) -> RoriIrcEntry {
        // Connect to IRC from config file
        let server = IrcServer::new(&config).unwrap();
        server.identify().unwrap();
        RoriIrcEntry { server: server }
    }

    // For each message received by the bot. Make actions.
    fn process_msg(&self, client: &mut RoriClient, incoming: &Arc<Mutex<Vec<String>>>) {
        for message in self.server.iter() {
            // TODO non blocking self.server.iter()
            if incoming.lock().unwrap().len() != 0 {
                self.write(incoming.lock().unwrap().pop());
            }
            self.msg_handle(&message.unwrap(), client);
        }
    }

    // Send private message, ping, etc.
    // TODO remove repeat
    fn msg_handle(&self, msg: &Message, client: &mut RoriClient) {
        let author = msg.source_nickname().unwrap_or("");
        let msg = msg.to_string();
        // Get if someone is talking
        let pos_privmsg = msg.find("PRIVMSG #").unwrap_or(msg.len());
        if pos_privmsg != msg.len() {
            // Get content
            let mut content = "";
            let pos_content = msg[1..].find(":").unwrap_or(msg.len());
            if pos_content != msg.len() && (pos_content + 2) < msg.len() {
                content = &msg[(pos_content + 2)..];
            }
            content = content.trim();
            // Send to RORI
            client.send_to_rori(&author, &content, "irc_entry_module", "text");
            println!("FROM: {} CONTENT: {}", &author, &content);
        }
    }

    fn write(&self, content: Option<String>) {
        match content {
            Some(s) => {
                self.server
                    .send_privmsg("#RORI", &s)
                    .unwrap();
            }
            None => {}
        }

    }
}

#[derive(Clone, RustcDecodable, RustcEncodable, Default, PartialEq, Debug)]
struct RoriServer {
    rori_ip: Option<String>,
    rori_port: Option<String>,
}

pub struct Endpoint {
    address: String,
    rori_address: String,
}

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
        let rori_address = Endpoint::parse_config_rori(data);
        if address == ":" || rori_address == ":" {
            println!("Empty config for the connection to the server");
        }
        Endpoint {
            address: address,
            rori_address: rori_address,
        }
    }

    pub fn start(&self, vec: Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind(&*self.address).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut client = Client::new(stream.try_clone().unwrap());
                    let content = client.read();
                    println!("RECEIVED:{}", &content);
                    let end = content.find(0u8 as char);
                    let (content, _) = content.split_at(end.unwrap_or(content.len()));
                    let data_to_process = RoriData::from_json(String::from(content));
                    if data_to_process.datatype == "text" {
                        println!("Push {}", &data_to_process.content);
                        vec.lock().unwrap().push(data_to_process.content);
                    }
                }
                Err(e) => {
                    println!("Connection failed because {}", e);
                }
            };
        }
        drop(listener);
    }

    pub fn register(&self) {
        // TODO security and if correctly registered
        let rori_address = self.rori_address.clone();
        let address = self.address.clone();
        let mut client = RoriClient { address: rori_address };
        let mut content = String::from(address);
        content.push_str("|");
        content.push_str("text");
        client.send_to_rori("AmarOk", &*content, "irc_entry_module", "register")
    }
}

// Launch RoriIrcEntry
fn main() {
    // will contains messages from RORI
    let vec = Arc::new(Mutex::new(Vec::new()));
    let vec_cloned = vec.clone();
    let child = thread::spawn(move || {
        let endpoint = Endpoint::new("config_endpoint.json");
        endpoint.register();
        endpoint.start(vec);
    });

    let rori = RoriIrcEntry::new("config_bot.json");
    let mut client = RoriClient::new("config_server.json");
    rori.process_msg(&mut client, &vec_cloned);
    let _ = child.join();
}
