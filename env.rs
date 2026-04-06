pub fn revision() -> Result<String, std::env::VarError> {
    std::env::var("REVISION")
}
