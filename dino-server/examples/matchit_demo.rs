use anyhow::Result;
use matchit::Router;

fn main() -> Result<()> {
    let mut router = Router::new();
    router.insert("/home", "Welcome!")?;
    router.insert("/users/{id}", "A User")?;

    let matched = router.at("/users/323")?;
    assert_eq!(matched.params.get("id"), Some("323"));
    assert_eq!(*matched.value, "A User");

    let matched = router.at("/home")?;
    assert_eq!(*matched.value, "Welcome!");

    let matched = router.at("/not-found");
    assert!(matched.is_err());
    Ok(())
}
