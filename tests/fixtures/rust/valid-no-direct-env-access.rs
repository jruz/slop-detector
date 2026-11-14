use std::env;

fn load_env() -> Result<String, std::env::VarError> {
    let token = std::env::var("TOKEN")?;
    let port = env::var("PORT")?;
    Ok(format!("{token}:{port}"))
}
