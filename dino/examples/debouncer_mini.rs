use std::{path::Path, time::Duration};

use notify::RecursiveMode;
use notify_debouncer_mini::{Config, new_debouncer_opt};

fn main() {
    std::thread::spawn(|| {
        let path = Path::new("test.txt");
        let _ = std::fs::write(path, "Hello, world!");
        loop {
            std::fs::write(path, b"contents").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    });

    let (tx, rx) = std::sync::mpsc::channel();
    let backend_cfg = notify::Config::default().with_poll_interval(Duration::from_secs(1));
    let debouncer_config = Config::default()
        .with_timeout(Duration::from_millis(1000))
        .with_notify_config(backend_cfg);

    let mut debouncer = new_debouncer_opt::<_, notify::PollWatcher>(debouncer_config, tx).unwrap();

    debouncer
        .watcher()
        .watch(Path::new("../dino/examples/"), RecursiveMode::Recursive)
        .unwrap();

    for result in rx {
        match result {
            Ok(events) => {
                for event in events {
                    println!("event: {:?}", event);
                }
            }
            Err(e) => println!("error: {:?}", e),
        }
    }
}
