use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow, Debug)]
pub struct Block {
    pub id: Uuid,
    pub chain_id: i16,
    pub height: i64,
}

impl Block {
    pub fn new(chain_id: u16, height: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            chain_id: chain_id as i16,
            height: height as i64,
        }
    }
    pub async fn insert(self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO blocks(id, chain_id, height)
            VALUES ($1, $2, $3)
            "#,
            self.id,
            self.chain_id,
            self.height
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn insert_as_last_processed_block(self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO processed_blocks_logs(id, chain_id, height)
            VALUES ($1, $2, $3)
            ON CONFLICT (chain_id)
            DO
                UPDATE SET height=EXCLUDED.height;
            "#,
            self.id,
            self.chain_id,
            self.height
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
