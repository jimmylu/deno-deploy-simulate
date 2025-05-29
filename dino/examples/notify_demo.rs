use notify::Event;
use notify::Result;
use notify::Watcher;
use std::path::Path;

fn main() -> Result<()> {
    // let (tx, rx) = mpsc::channel::<Result<Event>>();
    // let mut watcher = notify::recommended_watcher(tx)?;
    // watcher.watch(Path::new("."), notify::RecursiveMode::Recursive)?;
    // for res in rx {
    //     match res {
    //         Ok(event) => println!("event: {:?}", event),
    //         Err(e) => println!("error: {:?}", e),
    //     }
    // }

    let mut watcher = notify::recommended_watcher(event_fn)?;
    watcher.watch(Path::new("."), notify::RecursiveMode::Recursive)?;

    Ok(())
}

fn event_fn(res: Result<Event>) {
    match res {
        Ok(event) => println!("#event: {:?}", event),
        Err(e) => println!("#error: {:?}", e),
    }
}
