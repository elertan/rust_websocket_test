extern crate websocket;

use std::str;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;

use websocket::sync::Server;
use websocket::Message;
use websocket::sender::Sender;
use websocket::receiver::Receiver;
use websocket::header::WebSocketProtocol;
use websocket::message::Type;
use websocket::OwnedMessage;
use websocket::ws::dataframe::DataFrame;
use websocket::dataframe::Opcode;


fn main() {
    let server = Server::bind("0.0.0.0:2794").unwrap();

    let (dispatcher_tx, dispatcher_rx) = mpsc::channel::<String>();
    let client_senders: Arc<Mutex<Vec<mpsc::Sender<String>>>> = Arc::new(Mutex::new(vec![]));

    // dispatcher thread
    {
        let client_senders = client_senders.clone();
        thread::spawn(move || {
            while let Ok(msg) = dispatcher_rx.recv() {
                for sender in client_senders.lock().unwrap().iter() {
                    sender.send(msg.clone()).unwrap();
                }
            }
        });
    }

    // client threads
    for connection in server {
        let dispatcher = dispatcher_tx.clone();
        let (client_tx, client_rx) = mpsc::channel();
        client_senders.lock().unwrap().push(client_tx);

        // Spawn a new thread for each connection.
        thread::spawn(move || {
            let conn = connection.unwrap_or_else(|_| panic!("Unwrapping request failed")); // Get the request
            let headers = conn.request.headers.clone(); // Keep the headers so we can check them

//            request.validate().unwrap(); // Validate the request

            let mut response = conn.accept().unwrap_or_else(|_| panic!("asdasd")); // Form a response

//            if let Some(&WebSocketProtocol(ref protocols)) = headers.get() {
//                if protocols.contains(&("rust-websocket".to_string())) {
//                    // We have a protocol we want to use
//                    response.headers().set(WebSocketProtocol(vec!["rust-websocket".to_string()]));
//                }
//            }

//            let mut client = response.send_message().unwrap(); // Send the response
//
//            let ip = client.get_mut_sender()
//                .get_mut()
//                .peer_addr()
//                .unwrap();

            let ip = response.peer_addr().expect("Peer IP");

            println!("Connection from {}", ip);

            let message: Message = Message::text("SERVER: Connected.".to_string());
            response.send_message(&message).unwrap();

            let (mut receiver, mut sender) = response.split().expect("Split");

            let(tx, rx) = mpsc::channel::<OwnedMessage>();
            thread::spawn(move || {
                for message in receiver.incoming_messages() {
                    let msg = message.unwrap();
                    tx.send(msg).unwrap();
                }
            });

            loop {
                if let Ok(message) = rx.try_recv() {
                    match Opcode::new(message.opcode()).unwrap() {
                        Opcode::Close => {
                            let message = Message::close();
                            sender.send_message(&message).unwrap();
                            println!("Client {} disconnected", ip);
                            return;
                        },
                        Opcode::Ping => {
                            let message = Message::pong(message.take_payload());
                            sender.send_message(&message).unwrap();
                        },
                        _ => {
                            let payload_bytes = &message.take_payload();
                            let payload_string = match str::from_utf8(payload_bytes) {
                                Ok(v) => v,
                                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                            };
                            let msg_string = format!("MESSAGE: {}: ", payload_string);
                            dispatcher.send(msg_string).unwrap();
                        }
                    }
                }
                if let Ok(message) = client_rx.try_recv() {
                    let message: Message = Message::text(message);
                    sender.send_message(&message).unwrap();
                }
            }
        });
    }
}