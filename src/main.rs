extern crate irc;
extern crate rustc_serialize;

mod rori_utils;

use irc::client::prelude::*;
use std::path::Path;
use rori_utils::client::RoriClient;


// TODO get messages from Rori
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
    // TODO remove repeat
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
