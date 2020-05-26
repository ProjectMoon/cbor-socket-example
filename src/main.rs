use serde::{Deserialize, Serialize};
use std::env;
use std::io;
use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug)]
enum SocketMessage {
    Ping,
    Pong,
}

#[derive(Serialize, Deserialize, Error, Debug)]
enum CborSocketExampleError {
    #[error("something bad happened")]
    StuffBroke,

    #[error("cbor serialization went wrong")]
    CborSerializationError,

    #[error("there was an io error")]
    IoError { message: String },
}

impl From<serde_cbor::error::Error> for CborSocketExampleError {
    fn from(error: serde_cbor::error::Error) -> Self {
        CborSocketExampleError::CborSerializationError
    }
}

impl From<io::Error> for CborSocketExampleError {
    fn from(error: io::Error) -> Self {
        CborSocketExampleError::IoError {
            message: format!("{:?}", error),
        }
    }
}

type Result<T> = std::result::Result<T, CborSocketExampleError>;

#[derive(Serialize, Deserialize, Debug)]
struct SocketRequest {
    message: SocketMessage,
}

#[derive(Serialize, Deserialize, Debug)]
struct SocketResponse {
    message: SocketMessage,
    error: Option<CborSocketExampleError>,
}

fn send_message(socket_path: &Path) -> Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;
    let reader = stream.try_clone().unwrap();

    println!("Sending command");

    //This was originally to_writer
    let message = SocketRequest {
        message: SocketMessage::Ping,
    };

    serde_cbor::to_writer(&stream, &message)?;
    stream.flush()?;

    println!("Message sent. Wait 3 seconds...");

    //Another way that should produce the same result.
    // let vec_cmd = serde_cbor::to_vec(&message)?;
    // stream.write_all(&vec_cmd)?;
    // stream.flush()?;

    //Wait a bit before asking for response.
    use std::{thread, time};
    let time = time::Duration::from_secs(3);
    thread::sleep(time);

    //It will block here.
    println!("Receiving response");
    let response: SocketResponse = serde_cbor::from_reader(&stream)?;

    println!("Received: {:?}", response);
    Ok(())
}

//The error returned from this is about I/O errors (message sending/receiving),
//not about whether or not a command was handled properly.
//When this function exits, the socket connection close
//(because rust drops the stream, as it is no longer in scope).
//to keep it open, we'd have to loop here and read messages.
fn receive_message(stream: UnixStream) -> Result<()> {
    println!("Receiving message");

    //It will block here.
    let request: SocketRequest = serde_cbor::from_reader(&stream)?;
    println!("Message received: {:?}", request);

    let response = SocketResponse {
        message: SocketMessage::Pong,
        error: Some(CborSocketExampleError::StuffBroke),
    };

    println!("Sending response");
    serde_cbor::to_writer(&stream, &response)?;

    println!("Response sent");
    Ok(())
}

fn spawn_socket(socket_path: &Path) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let listener = UnixListener::bind(socket_path)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let handler_result = crossbeam::scope(|s| {
                    s.spawn(move |_| receive_message(stream));
                });

                match handler_result {
                    Ok(_) => println!("I/O processing of message was ok"),
                    Err(err) => println!("I/O error processing message: {:?}", err),
                }
            }
            Err(err) => {
                println!("Ending socket listening: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<String>>();

    match &args[..] {
        [_, cmd, socket_path] if cmd == "serve" => spawn_socket(Path::new(socket_path)),
        [_, cmd, socket_path] if cmd == "connect" => send_message(Path::new(socket_path)),
        _ => Err(CborSocketExampleError::StuffBroke),
    }
}
