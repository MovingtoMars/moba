use serde_json;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::sync::{Arc, Mutex};
use std::net::TcpStream;
use std::io::{self, Read, Write};
use std::thread;
use chan;
use common::{Command, EntityID, Event, Team};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Connect { name: String, team: Option<Team> },
    AcceptConnection { message: String },
    Ping { id: u64 },
    ReturnPing { id: u64 },
    Kick { reason: String },
    Quit,
    SendChat { message: String },
    ReceiveChat { user: String, message: String },
    Command(Command),
    CommandByPlayer { command: Command, player: EntityID },
    Events(Vec<Event>),
    SetPlayerEntityID(EntityID),
}

#[derive(Clone)]
pub struct Stream {
    reader: Arc<Mutex<TcpStream>>,
    writer: Arc<Mutex<TcpStream>>,
    incoming: chan::Receiver<io::Result<Message>>,
}

fn read_packet(stream: &mut TcpStream) -> io::Result<String> {
    let size = stream.read_u32::<BigEndian>()?;

    let mut buf = vec![0; size as usize];

    stream.read_exact(&mut buf)?;

    Ok(String::from_utf8(buf).unwrap()) // XXX
}

impl Stream {
    pub fn new(inner: TcpStream) -> Self {
        let (send, recv) = chan::sync(32);

        inner.set_nodelay(true).unwrap();
        let stream = Stream {
            writer: Arc::new(Mutex::new(inner.try_clone().unwrap())),
            reader: Arc::new(Mutex::new(inner)),
            incoming: recv,
        };

        {
            let reader = stream.reader.clone();
            thread::spawn(move || loop {
                let packet = read_packet(&mut reader.lock().unwrap());
                let packet = packet.map(|packet| decode_message(&packet));

                match packet {
                    Ok(Message::Quit) |
                    Err(_) => {
                        send.send(packet);
                        // send.send(Err(io::Error::from(io::ErrorKind::UnexpectedEof)));
                        return;
                    }
                    _ => send.send(packet),
                }
            });
        }

        stream
    }

    pub fn write_message(&mut self, message: Message) -> io::Result<()> {
        self.write_packet(&encode_message(message))
    }

    pub fn try_get_message(&self) -> Option<io::Result<Message>> {
        let inc = &self.incoming;
        chan_select! {
            default => return None,
            inc.recv() -> val => return Some(val.unwrap()),
        };
    }

    pub fn get_message(&self) -> io::Result<Message> {
        self.incoming.recv().unwrap()
    }

    fn write_packet(&mut self, s: &str) -> io::Result<()> {
        let mut writer = self.writer.lock().unwrap();

        assert!(s.len() <= u32::max_value() as usize);

        writer.write_u32::<BigEndian>(s.len() as u32)?;

        writer.write_all(s.as_bytes())?;

        Ok(())
    }
}

pub fn decode_message(s: &str) -> Message {
    let message: Message = serde_json::from_str(&s).unwrap();
    // println!("DESERIALISED: {:?}", message);
    message
}

pub fn encode_message(message: Message) -> String {
    let s = serde_json::to_string(&message).unwrap();
    // println!("SERIALISED: {}", s);
    s
}
