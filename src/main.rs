extern crate irc;
extern crate rustc_serialize;
// TODO get messages from Rori
// TODO sslstream

use irc::client::prelude::*;
use std::path::Path;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};

use std::net::TcpStream;
use std::fs::File;
use rustc_serialize::json::decode;

pub struct RoriTextData {
    author: String,
    content: String,
    client: String,
}

impl RoriTextData {
    pub fn new(author: String, content: String, client: String) -> RoriTextData {
        RoriTextData {
            author: author.replace("\"", "\\\""),
            content: content.replace("\"", "\\\""),
            client: client.replace("\"", "\\\""),
        }
    }

    pub fn to_string(&self) -> String {
        format!("{{
            \"author\":\"{}\",
            \"content\":\"{}\",
            \"client\":\"{}\",
        }}",
                self.author,
                self.content,
                self.client)
    }
}


pub struct RoriClient {
    stream: Option<TcpStream>,
}


#[derive(Clone, RustcDecodable, RustcEncodable, Default, PartialEq, Debug)]
pub struct ConfigServer {
    pub ip: Option<String>,
    pub port: Option<String>,
}

impl RoriClient {
    pub fn parse_config(data: String) -> String {
        let params: ConfigServer = decode(&data[..])
            .map_err(|_| {
                Error::new(ErrorKind::InvalidInput,
                           "Failed to decode configuration file.")
            })
            .unwrap();

        format!("{}:{}",
                &params.ip.unwrap_or(String::from("")),
                &params.port.unwrap_or(String::from("")))
    }

    pub fn new<P: AsRef<Path>>(config: P) -> RoriClient {
        // TODO configure from file
        let mut file = File::open(config)
            .ok()
            .expect("Config file not found");
        let mut data = String::new();
        file.read_to_string(&mut data)
            .ok()
            .expect("failed to read!");
        let address = RoriClient::parse_config(data);
        if address == ":" {
            println!("Empty config for the connection to the server");
        }
        let stream = Some(TcpStream::connect(&*address).unwrap());
        RoriClient { stream: stream }
    }

    pub fn send_to_rori(&mut self, author: &str, content: &str) {
        // TODO data_to_send in json
        let data_to_send = RoriTextData::new(String::from(author),
                                             String::from(content),
                                             String::from("irc_entry_module"));
        if let Some(ref mut stream) = self.stream {
            let _ = stream.write(data_to_send.to_string().as_bytes());
        } else {
            println!("Stream not initialized...");
        }
    }
}

// TODO move to file
pub struct RoriIrcEntry {
    server: IrcServer,
}

impl RoriIrcEntry {
    pub fn new<P: AsRef<Path>>(config: P) -> RoriIrcEntry {
        // Connect to IRC from config file
        let server = IrcServer::new(&config).unwrap();
        server.identify().unwrap();
        RoriIrcEntry { server: server }
    }

    // For each message received by the bot. Make actions.
    pub fn process_msg(&self, client: &mut RoriClient) {
        for message in self.server.iter() {
            self.msg_handle(&message.unwrap(), client);
        }
    }

    // Send private message, ping, etc.
    // TODO improve
    pub fn msg_handle(&self, msg: &Message, client: &mut RoriClient) {
        let author = msg.source_nickname().unwrap_or("");
        let msg = msg.to_string();
        // Get if we are in a channel
        let pos_channel = msg.find("#").unwrap_or(msg.len());
        let mut final_channel = "";
        if pos_channel != msg.len() {
            final_channel = &msg[pos_channel..];
            let pos_end_channel = final_channel.find(" ").unwrap_or(final_channel.len());
            final_channel = &final_channel[..pos_end_channel];
        }
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
            // Repeat msg to server
            self.server
                .send_privmsg(final_channel, &content)
                .unwrap();
            // Send to RORI
            client.send_to_rori(&author, &content);
            println!("FROM: {} CONTENT: {}", &author, &content);
        }
    }
}

// Launch RoriIrcEntry
fn main() {
    let rori = RoriIrcEntry::new("config_bot.json");
    let mut client = RoriClient::new("config_server.json");
    rori.process_msg(&mut client);
}
