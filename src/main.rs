extern crate env_logger;
extern crate irc;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate regex;

mod rori_utils;
mod endpoint;

use endpoint::Endpoint;
use irc::client::prelude::*;
use rori_utils::client::RoriClient;
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
    server: IrcServer,
    channels: Channels,
}

impl RoriIrcEntry {
    fn new<P: AsRef<Path>>(config: P) -> RoriIrcEntry {
        info!(target: "RoriIrcEntry", "init");
        // Connect to IRC from config file
        let server = IrcServer::new(&config).unwrap();
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
        for message in self.server.iter() {
            self.msg_handle(&message.unwrap(), client);
        }
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
            client.send_to_rori(&author, &content, "irc_entry_module", "text");
            info!(target:"RoriIrcEntry", "received message from {}: {}", &author, &content);
        }
    }
}

fn main() {
    // Init logging
    env_logger::init().unwrap();

    // will contains messages from RORI
    let incoming = Arc::new(Mutex::new(Vec::new()));
    let incoming_cloned = incoming.clone();
    let child_endpoint = thread::spawn(move || {
        let mut endpoint = Endpoint::new("config_endpoint.json");
        endpoint.register();
        if endpoint.is_registered {
            endpoint.start(incoming);
        } else {
            error!(target: "endpoint", "endpoint is not registered");
        }
    });

    let rori = RoriIrcEntry::new("config_bot.json");
    let mut client = RoriClient::new("config_server.json");
    rori.process_msg(&mut client, incoming_cloned);
    let _ = child_endpoint.join();
}
