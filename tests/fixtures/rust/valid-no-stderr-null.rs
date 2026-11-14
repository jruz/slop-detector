fn example() {
    Command::new("cmd")
        .stderr(Stdio::piped())
        .spawn();
}
