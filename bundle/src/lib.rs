mod bundle;
use anyhow::Result;
pub use bundle::*;

pub type ModulePath = String;
pub type ModuleSource = String;

/// Defines the interface of a module loader.
pub trait ModuleLoader {
    fn load(&self, specifier: &str) -> Result<ModuleSource>;
    fn resolve(&self, base: Option<&str>, specifier: &str) -> Result<ModulePath>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_ts_should_work() {
        let entry = "fixtures/main.ts";
        let ret = run_bundle(entry, &Default::default()).unwrap();

        assert_eq!(
            ret,
            "(function(){async function execute(name){console.log(\"Executing lib.\");return`Hello ${name}!`;}async function main(){console.log(\"Executing main.\");console.log(await execute(\"world\"));}return{default:main};})();"
        );
    }
}
