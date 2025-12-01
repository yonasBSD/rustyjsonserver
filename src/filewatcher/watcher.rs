use tokio::sync::mpsc;
use tokio::runtime::Handle;
use notify::{recommended_watcher, RecursiveMode, Watcher, Event};
use tracing::{error, info};

use crate::config::manager::ConfigManager;

pub fn spawn_watcher(manager: ConfigManager) {
    let rt_handle = Handle::current();

    tokio::spawn(async move {
        let (tx, mut rx) = mpsc::channel::<notify::Result<Event>>(100);
        let mut watcher = recommended_watcher(move |res| {
            let tx_clone = tx.clone();
            let handle = rt_handle.clone();
            handle.spawn(async move {
                if tx_clone.send(res).await.is_err() {
                    info!("File watcher event channel closed");
                }
            });
        }).expect("Failed to initialize file watcher");

        watcher.watch(manager.root_folder(), RecursiveMode::Recursive)
               .expect("Failed to watch configuration folder");
        info!("Watching project files for changes...");

        while let Some(res) = rx.recv().await {
            match res {
                Ok(event) => {
                    info!(?event, "File change detected");
                    if let Err(err) = manager.reload() {
                        error!(error = %err, "Configuration reload error");
                    }
                }
                Err(err) => error!(error = ?err, "Watch error")
            }
        }
    });
}