func fetchData() -> String? {
    let data = try? loadFromDisk()
    return data
}
