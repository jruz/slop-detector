func example1() {
    do {
        try loadData()
    } catch { }
}

func example2() {
    do {
        try loadData()
    } catch { return }
}

func example3() -> String? {
    do {
        return try loadData()
    } catch { return nil }
}
