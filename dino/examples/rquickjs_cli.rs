use std::io::Write;

use rquickjs::{CatchResultExt, Context, Function, Object, Result, Runtime, Value};

fn print(msg: String) {
    println!("{}", msg);
}
fn main() -> Result<()> {
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;

    ctx.with(|ctx| -> Result<()> {
        let global = ctx.globals();
        global.set(
            "__print",
            // 将rust的print函数绑定到globalThis.__print
            Function::new(ctx.clone(), print)?.with_name("__print")?,
        )?;
        // 将rust的print函数绑定到globalThis.console.log
        ctx.eval::<(), _>(
            r#"
globalThis.console = {
  log(...v) {
    globalThis.__print(`${v.join(" ")}`)
  }
}
"#,
        )?;

        let console: Object = global.get("console")?;
        let js_log: Function = console.get("log")?;
        loop {
            let mut input = String::new();
            print!("> ");
            std::io::stdout().flush()?;
            std::io::stdin().read_line(&mut input)?;
            ctx.eval::<Value, _>(input.as_bytes())
                .and_then(|ret| js_log.call::<(Value<'_>,), ()>((ret,)))
                .catch(&ctx)
                .unwrap_or_else(|err| println!("{err}"));
        }
    })?;

    Ok(())
}
