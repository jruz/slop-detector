func fetchData() -> String? {
    guard let value = loadData() else { return nil }
    return value
}
