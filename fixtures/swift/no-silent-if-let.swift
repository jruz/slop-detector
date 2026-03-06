func fetchData() {
    if let value = loadData() {
        process(value)
    } else { return }
}
