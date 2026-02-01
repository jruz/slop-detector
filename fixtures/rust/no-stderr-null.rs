fn example() {
    Command::new("cmd")
        .stderr(Stdio::null())
        .spawn();
}
