use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc::Sender, Notify},
    task::JoinHandle,
};

#[tokio::main]
async fn main() {
    struct PortTarget {
        port: u16,
        target: String,
    }

    impl PortTarget {
        fn new(port: u16, target: String) -> Self {
            PortTarget { port, target }
        }
    }

    // mockdata - Fetch this from sqlite in the future. NOTE - maybe use a .yaml file instead?
    let port_targets: Vec<PortTarget> = vec![
        PortTarget::new(3000, "http://localhost:3000".to_string()),
        PortTarget::new(3001, "http://localhost:3001".to_string()),
        PortTarget::new(3002, "http://localhost:3002".to_string()),
        PortTarget::new(3003, "http://localhost:3003".to_string()),
    ];

    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let rx = Arc::new(tokio::sync::Mutex::new(rx));
    let notify = Arc::new(Notify::new());

    let mut tasks: Vec<JoinHandle<()>> = Vec::new();
    let number_of_workers = 4;

    for port_target in port_targets {
        let tx: Sender<(TcpStream, String)> = tx.clone();
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port_target.port))
            .await
            .unwrap();
        let notify = notify.clone();

        tasks.push(tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, _)) => {
                        let notify = notify.clone();

                        let message = (socket, port_target.target.clone());

                        if let Err(e) = tx.send(message).await {
                            eprintln!("Failed to send message: {}", e);
                        }

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
                while let Ok((mut socket, target)) = rx.try_recv() {
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

                                    println!("worker {} active", worker)
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
