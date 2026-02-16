use sqlx::postgres::PgPoolOptions;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "postgres://postgres:postgres@localhost:5432/postgres";
    let pool = PgPoolOptions::new().connect(url).await?;
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await?;
    println!("Connected! {}", row.0);
    Ok(())
}
