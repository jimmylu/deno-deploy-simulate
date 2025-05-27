use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;

fn main() -> Result<()> {
    let config = ArcSwap::from(Arc::new(String::default()));
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let new_conf = Arc::new("new configuration".to_owned());
            config.store(new_conf);
        });

        for i in 0..10 {
            println!("spawned thread {}", i);
            scope.spawn(|| {
                loop {
                    let cfg = config.load();
                    println!("cfg: {:?}", cfg);
                    if !cfg.is_empty() {
                        assert_eq!(**cfg, "new configuration");
                        return;
                    }
                }
            });
        }
    });
    Ok(())
}
