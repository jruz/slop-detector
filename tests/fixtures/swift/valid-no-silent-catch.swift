func fetchData() {
    do {
        try loadData()
    } catch {
        print(error)
    }
}
