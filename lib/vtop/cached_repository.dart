abstract class CachedRepository<T> {
  Future<T?> loadCache();
  Future<void> saveCache(T data);
  Future<T> fetchRemote();

  Future<T> load() async {
    final cached = await loadCache();
    if (cached != null) return cached;
    return refresh();
  }

  Future<T> refresh() async {
    final remote = await fetchRemote();
    await saveCache(remote);
    return remote;
  }
}
