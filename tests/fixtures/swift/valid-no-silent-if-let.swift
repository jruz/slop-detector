func fetchData() throws {
    if let value = loadData() {
        process(value)
    } else {
        throw FetchError.notFound
    }
}
