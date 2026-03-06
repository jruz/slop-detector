func fetchData() throws -> String {
    guard let value = loadData() else { throw FetchError.notFound }
    return value
}
