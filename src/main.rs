extern crate crypto;
extern crate env_logger;
extern crate irc;
#[macro_use]
extern crate log;
extern crate openssl;
extern crate rustc_serialize;
extern crate regex;

mod rori_utils;
mod endpoint;

use endpoint::IRCEndpoint;
use irc::client::prelude::*;
use rori_utils::client::RoriClient;
use rori_utils::endpoint::Endpoint;
use rustc_serialize::json::decode;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone, RustcDecodable, RustcEncodable, Default, PartialEq, Debug)]
struct Channels {
    channels: Vec<String>,
}

/**
 * RoriIrcEntry is an IRC bot for RORI
 */
struct RoriIrcEntry {
    server: IrcClient,
    channels: Channels,
    secret: String,
    name: String,
}

impl RoriIrcEntry {
    fn new<P: AsRef<Path>>(config: P, secret: String, name: String) -> RoriIrcEntry {
        info!(target: "RoriIrcEntry", "init");
        // Connect to IRC from config file
        let server = IrcClient::from_config(Config::load(&config).unwrap()).unwrap();
        // Configure from file
        let mut file = File::open(config)
            .ok()
            .expect("Config file not found");
        let mut data = String::new();
        file.read_to_string(&mut data)
            .ok()
            .expect("failed to read!");
        let channels: Channels = decode(&data[..]).unwrap();
        server.identify().unwrap();
        RoriIrcEntry {
            server: server,
            channels: channels,
            secret: secret,
            name: name,
        }
    }

    /**
     * For each message received by the bot. Make actions.
     * @param: client: where we send messages from IRC
     * @param: incoming: what the bot says on IRC
     */
    fn process_msg(&self, client: &mut RoriClient, incoming: Arc<Mutex<Vec<String>>>) {
        info!(target: "RoriIrcEntry", "process_msg");
        // Send incoming messages to IRC
        let cloned_serv = self.server.clone();
        let channels_cloned = self.channels.clone();
        thread::spawn(move || {
            loop {
                if incoming.lock().unwrap().len() != 0 {
                    match incoming.lock().unwrap().pop() {
                        Some(s) => {
                            info!(target:"RoriIrcEntry", "write: {}", &s);
                            cloned_serv.send_privmsg(&*channels_cloned.channels[0], &s)
                                .unwrap();
                        }
                        None => {}
                    }
                }
            }
        });
        // Get message from IRC
        self.server.for_each_incoming(|message| {
            self.msg_handle(&message, client);
        }).unwrap();
    }

    /**
     * Handle message from IRC
     * @param: msg the Message from IRC
     * @param: client to send RORIData to RORI
     */
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
            client.send_to_rori(&author, &content, &self.name, "text", &self.secret);
            info!(target:"RoriIrcEntry", "received message from {}: {}", &author, &content);
        }
    }
}


#[derive(Clone, RustcDecodable, RustcEncodable, Default, PartialEq, Debug)]
pub struct Details {
    pub secret: Option<String>,
    pub name: Option<String>,
}

fn main() {
    // Init logging
    env_logger::init();

    // will contains messages from RORI
    let incoming = Arc::new(Mutex::new(Vec::new()));
    let incoming_cloned = incoming.clone();
    let child_endpoint = thread::spawn(move || {
        let mut endpoint = IRCEndpoint::new("config_endpoint.json", incoming);
        endpoint.register();
        if endpoint.is_registered() {
            endpoint.start();
        } else {
            error!(target: "endpoint", "endpoint is not registered");
        }
    });

    let mut file = File::open("config_endpoint.json")
        .ok()
        .expect("Config file not found");
    let mut data = String::new();
    file.read_to_string(&mut data)
        .ok()
        .expect("failed to read!");
    let details: Details = decode(&data[..]).unwrap();

    let mut client = RoriClient::new("config_server.json");
    let rori = RoriIrcEntry::new("config_bot.json",
                                 details.secret.unwrap_or(String::from("")),
                                 details.name.unwrap_or(String::from("")));
    rori.process_msg(&mut client, incoming_cloned);
    let _ = child_endpoint.join();
}
