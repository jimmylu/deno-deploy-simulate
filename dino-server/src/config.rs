use crate::ProjectRoutes;
use anyhow::Result;
use axum::http::Method;
use serde::{Deserialize, Deserializer};

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub routes: ProjectRoutes,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct ProjectRoute {
    #[serde(deserialize_with = "deserialize_method")]
    pub method: Method,
    pub handler: String,
}

// 自定义方法的反序列化 fn<'de, D>(D) -> Result<T, D::Error> where D: Deserializer<'de>
fn deserialize_method<'de, D>(deserializer: D) -> Result<Method, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.to_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "DELETE" => Ok(Method::DELETE),
        "PATCH" => Ok(Method::PATCH),
        "HEAD" => Ok(Method::HEAD),
        "OPTIONS" => Ok(Method::OPTIONS),
        "CONNECT" => Ok(Method::CONNECT),
        "TRACE" => Ok(Method::TRACE),
        _ => Err(serde::de::Error::custom("invalid method")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;

    #[test]
    fn test_deserialize_method_should_work() -> Result<()> {
        let rdr = File::open("fixtures/config.yml")?;
        let config: ProjectConfig = serde_yaml::from_reader(rdr)?;
        assert_eq!(config.name, "dino-test");
        assert_eq!(config.routes.len(), 2);
        assert_eq!(config.routes.get("/api/hello/{id}").unwrap().len(), 2);
        assert_eq!(config.routes.get("/api/{name}/{id}").unwrap().len(), 2);
        println!("{:?}", config.routes);

        Ok(())
    }
}
