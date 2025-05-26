use std::fs;

use anyhow::Result;
use quote::quote;

fn main() -> Result<()> {
    // let input = r#"
    // #[derive(Debug)]
    // pub struct Person {
    //     name: String,
    //     age: u32,
    // }
    // "#;

    // let ast = syn::parse_str::<syn::ItemStruct>(input)?;
    // println!("{:#?}", ast);

    let file = fs::read_to_string("src/lib.rs")?;
    let ast = syn::parse_file(&file)?;

    let tokens = quote! { #ast };
    println!("{:#?}", tokens);

    Ok(())
}
