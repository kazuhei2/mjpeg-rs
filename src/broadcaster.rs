use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Bytes, Data};
use actix_web::Error;

use tokio::prelude::*;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use std::fs::File;
use std::thread::sleep;
use std::time;
use std::sync::Mutex;


/// Hold clients channels
pub struct Broadcaster {
    clients: Vec<Sender<Bytes>>,
}

impl Broadcaster {
    fn new() -> Self {
        Broadcaster {
            clients: Vec::new(),
        }
    }

    pub fn create(width: u32, height: u32, fps: u64) -> Data<Mutex<Self>> {
        // Data ≃ Arc
        let me = Data::new(Mutex::new(Broadcaster::new()));

        Broadcaster::spawn_capture(me.clone(), width, height, fps);

        me
    }

    pub fn new_client(&mut self) -> Client {
        let (tx, rx) = channel(100);

        self.clients.push(tx);
        Client(rx)
    }

    #[cfg(target_os = "linux")]
    fn make_message_block(frame: &[u8]) -> Vec<u8> {
        let mut msg = format!(
            "--boundarydonotcross\r\nContent-Length:{}\r\nContent-Type:image/jpeg\r\n\r\n",
            frame.len()
        )
        .into_bytes();
        msg.extend(frame);
        msg
    }

    fn send_image(&mut self, msg: &[u8]) {
        let mut ok_clients = Vec::new();
        for client in self.clients.iter() {
            let result = client.clone().try_send(Bytes::from(&msg[..]));

            if let Ok(()) = result {
                ok_clients.push(client.clone());
            }
        }
        self.clients = ok_clients;
    }

    #[cfg(target_os = "linux")]
    fn spawn_capture(me: Data<Mutex<Self>>, width: u32, height: u32, fps: u64) {
        let mut count = 0;
        std::thread::spawn(move || loop {
            count += 1;
            let path = format!("../../jpeg/risa_{:03}.jpg", count);
            let mut f = File::open(&path).unwrap();
            if count == 603 { count = 0; }
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            let msg = Broadcaster::make_message_block(&buf);
            me.lock().unwrap().send_image(&msg);
            let wait_ms = time::Duration::from_millis(33);
            sleep(wait_ms);
        });
    }
}

// wrap Receiver in own type, with correct error type
pub struct Client(Receiver<Bytes>);

impl Stream for Client {
    type Item = Bytes;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.0.poll().map_err(ErrorInternalServerError)
    }
}
