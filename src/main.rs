use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{Receiver, Sender},
        Notify,
    },
    task::JoinHandle,
};

#[tokio::main]
async fn main() {
    // mockdata - Fetch this from sqlite in the future. NOTE - maybe use a .yaml file instead?
    let ports: Vec<u16> = vec![3037, 3034, 3035, 3036];

    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let rx = Arc::new(tokio::sync::Mutex::new(rx));
    let notify = Arc::new(Notify::new());

    let mut tasks: Vec<JoinHandle<()>> = Vec::new();
    let number_of_workers = 4;

    for port in ports {
        let tx: Sender<TcpStream> = tx.clone();
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let notify = notify.clone();

        tasks.push(tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, _)) => {
                        let notify = notify.clone();

                        if let Err(e) = tx.send(socket).await {
                            eprintln!("Failed to send message: {}", e);
                        }

                        println!("port: {} - pong", port);

                        notify.notify_one();
                    }
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                    }
                }
            }
        }));
    }

    for worker in 0..number_of_workers {
        let rx = Arc::clone(&rx);
        let notify = notify.clone();

        tokio::spawn(async move {
            loop {
                notify.notified().await;
                let mut rx = rx.lock().await;
                while let Ok(mut socket) = rx.try_recv() {
                    tokio::spawn(async move {
                        let mut buf = [0; 1024];

                        loop {
                            match socket.read(&mut buf).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    if let Err(e) = socket.write_all(&buf[0..n]).await {
                                        eprintln!("Failed to write data: {}", e);
                                        break;
                                    }

                                    println!("ping {}", worker)
                                }
                                Err(e) => {
                                    eprintln!("Failed to read from socket: {}", e);
                                    break;
                                }
                            };
                        }
                    });
                }
            }
        });
    }

    for task in tasks {
        task.await;
    }
}
