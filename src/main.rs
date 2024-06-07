use std::sync::Arc;

use tokio::{
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

        tasks.push(tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, _)) => {
                        if let Err(e) = tx.send(socket).await {
                            eprintln!("Failed to send message: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                    }
                }
            }
        }));
    }

    async fn handle_connection(socket: TcpStream) {}

    for _ in 0..number_of_workers {
        let mut rx = Arc::clone(&rx);

        let notify = notify.clone();

        tokio::spawn(async move {
            loop {
                notify.notified().await;
            }
        });
    }

    for task in tasks {
        task.await;
    }
}
