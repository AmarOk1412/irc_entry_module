use openssl::ssl::{SslContext, SslMethod, SslStream, SSL_VERIFY_NONE};
use openssl::x509::X509FileType::PEM;
use rori_utils::data::RoriData;
use rori_utils::endpoint::{Endpoint, Client, RoriEndpoint};
use std::path::Path;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

pub struct IRCEndpoint {
    endpoint: RoriEndpoint,
    incoming_data: Arc<Mutex<Vec<String>>>,
}

#[allow(dead_code)]
/**
 * Handle data from RORI and store it
 */
impl Endpoint for IRCEndpoint {
    fn start(&self) {
        let vec = self.incoming_data.clone();
        let listener = TcpListener::bind(&*self.endpoint.address).unwrap();
        let mut ssl_context = SslContext::new(SslMethod::Tlsv1).unwrap();
        // Enable TLS
        match ssl_context.set_certificate_file(&*self.endpoint.cert.clone(), PEM) {
            Ok(_) => info!(target:"Server", "Certificate set"),
            Err(_) => error!(target:"Server", "Can't set certificate file"),
        };
        ssl_context.set_verify(SSL_VERIFY_NONE, None);
        match ssl_context.set_private_key_file(&*self.endpoint.key.clone(), PEM) {
            Ok(_) => info!(target:"Server", "Private key set"),
            Err(_) => error!(target:"Server", "Can't set private key"),
        };

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let ssl_stream = SslStream::accept(&ssl_context, stream.try_clone().unwrap());
                    let ssl_ok = match ssl_stream {
                        Ok(_) => true,
                        Err(_) => false,
                    };
                    if ssl_ok {
                        let ssl_stream = ssl_stream.unwrap();
                        let mut client = Client::new(ssl_stream.try_clone().unwrap());
                        let content = client.read();
                        info!(target:"endpoint", "Received:{}", &content);
                        let end = content.find(0u8 as char);
                        let (content, _) = content.split_at(end.unwrap_or(content.len()));
                        // Get data from RORI
                        let data_to_process = RoriData::from_json(String::from(content));
                        let data_authorized = self.is_authorized(data_to_process.clone());
                        if data_authorized {
                            if data_to_process.datatype == "text" {
                                vec.lock().unwrap().push(data_to_process.content);
                            }
                        } else {
                            error!(target:"Server", "Stream not authorized! Don't process.");
                        }
                    } else {
                        error!(target:"Server", "Can't create SslStream");
                    }
                }
                Err(e) => {
                    error!(target:"endpoint", "{}", e);
                }
            };
        }
        drop(listener);
    }

    fn is_authorized(&self, data: RoriData) -> bool {
        self.endpoint.is_authorized(data)
    }

    fn register(&mut self) {
        self.endpoint.register()
    }
}

impl IRCEndpoint {
    pub fn new<P: AsRef<Path>>(config: P, incoming_data: Arc<Mutex<Vec<String>>>) -> IRCEndpoint {
        IRCEndpoint {
            endpoint: RoriEndpoint::new(config),
            incoming_data: incoming_data,
        }
    }

    pub fn is_registered(&self) -> bool {
        self.endpoint.is_registered
    }
}
