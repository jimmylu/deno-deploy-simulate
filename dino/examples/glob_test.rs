use anyhow::Result;
// use glob::glob;
use glob::MatchOptions;
use glob::glob_with;
fn main() -> Result<()> {
    //print all rs files in src/ directory
    // let paths = glob("src/**/*.rs")?;
    // for entry in paths {
    //     match entry {
    //         Ok(path) => println!("{:?}", path.display()),
    //         Err(e) => println!("Error: {}", e),
    //     }
    // }

    //print all *m*.rs files in src/ directory
    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    let paths = glob_with("src/**/*u*.rs", options)?;
    for entry in paths {
        match entry {
            Ok(path) => println!("{:?}", path.display()),
            Err(e) => println!("Error: {}", e),
        }
    }
    Ok(())
}
