#[cfg(test)]
mod tests {
    use taos::{AsyncQueryable, RawResult, Taos};
    use taos_macros::test;

    #[test]
    fn sync() {}

    #[test]
    async fn async_unit() {}

    #[test]
    async fn async_with_taos(_taos: &Taos) {}

    #[test]
    async fn async_with_taos_db(_taos: &Taos, _database: &str) {}

    #[test(databases = 10)]
    async fn async_with_taos_multi(taos: &Taos, _databases: &[&str]) -> RawResult<()> {
        let a = taos.databases().await?;
        assert!(a.len() >= 10);
        Ok(())
    }
}
