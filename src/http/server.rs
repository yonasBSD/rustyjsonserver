use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tracing::{error, info};
use super::{handler::handle_client, router::RoutesData};

pub async fn run(
    address: &str,
    routes: Arc<RwLock<Option<RoutesData>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(address).await?;
    info!("Server listening on {}", address);

    loop {
        let (stream, _) = listener.accept().await?;
        let routes_clone = Arc::clone(&routes);
        tokio::spawn(async move {
            let snapshot = {
                let guard = routes_clone.read().unwrap();
                guard.clone()
            };
            if let Err(e) = handle_client(stream, snapshot).await {
                error!("Error handling client: {}", e);
            }
        });
    }
}
