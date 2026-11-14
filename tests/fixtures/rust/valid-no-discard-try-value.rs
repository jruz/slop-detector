use std::process::Command;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("ls")
        .arg("-la")
        .output()?;
    println!("{}", output.status);

    Command::new("echo").arg("hi").output()?;

    fallible()?;

    let value = something.execute()?;
    println!("{value}");

    Ok(())
}
