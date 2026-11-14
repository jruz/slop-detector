use std::process::Command;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _output = Command::new("ls")
        .arg("-la")
        .output()?;

    let _ = Command::new("rm")
        .arg("temp")
        .output()?;

    let _resp = client.post(url).send().await?;

    Ok(())
}
