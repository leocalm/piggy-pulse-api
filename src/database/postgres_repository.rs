use deadpool_postgres::Client;

pub struct PostgresRepository<'a> {
    pub client: &'a Client,
}
