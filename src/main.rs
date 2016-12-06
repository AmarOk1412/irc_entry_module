extern crate irc;

use irc::client::prelude::*;
use std::path::Path;

pub struct RoriIrcEntry {
    server: IrcServer,
}

impl RoriIrcEntry {
    pub fn new<P: AsRef<Path>>(config: P) -> RoriIrcEntry {
        // Create the IRC Server
        let server = IrcServer::new(&config).unwrap();
        server.identify().unwrap();

        // Create the object
        RoriIrcEntry { server: server }
    }

    // For each message received by the bot. Make actions.
    pub fn process_msg(&self) {
        for message in self.server.iter() {
            self.msg_handle(&message.unwrap());
        }
    }

    // Send private message, ping, etc.
    pub fn msg_handle(&self, msg: &Message) {
        let msg = msg.to_string();
        print!("Message reçu : {}", &msg);
        let pos_channel = msg.find("#").unwrap_or(msg.len());
        let mut final_channel = "";
        if pos_channel != msg.len() {
            final_channel = &msg[pos_channel..];
            let pos_end_channel = final_channel.find(" ").unwrap_or(final_channel.len());
            final_channel = &final_channel[..pos_end_channel];
        }
        let pos_privmsg = msg.find("PRIVMSG #").unwrap_or(msg.len());
        if pos_privmsg != msg.len() {
            // Repeat msg to server
            self.server
                .send_privmsg(final_channel, &msg)
                .unwrap();
        }
    }
}

// Launch Medic
fn main() {
    let rori = RoriIrcEntry::new("config.json");
    rori.process_msg();
}
