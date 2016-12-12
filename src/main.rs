extern crate irc;
extern crate rustc_serialize;

mod rori_utils;

use irc::client::prelude::*;
use std::io::prelude::*;
use std::path::Path;
use std::net::{TcpListener, TcpStream};
use std::str::from_utf8;
use rori_utils::client::RoriClient;
use std::thread;
use std::sync::{Arc, Mutex};

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
            client.send_to_rori(&author, &content);
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

// Launch RoriIrcEntry
fn main() {
    // will contains messages from RORI
    let vec = Arc::new(Mutex::new(Vec::new()));
    let vec_cloned = vec.clone();
    // TODO server from config file
    let child = thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:1413").unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut client = Client::new(stream.try_clone().unwrap());
                    let content = client.read();
                    println!("RECEIVED:{}", &content);
                    vec.lock().unwrap().push(content);
                }
                Err(e) => {
                    println!("Connection failed because {}", e);
                }
            };
        }
        drop(listener);
    });

    let rori = RoriIrcEntry::new("config_bot.json");
    let mut client = RoriClient::new("config_server.json");
    rori.process_msg(&mut client, &vec_cloned);
    let _ = child.join();
}
