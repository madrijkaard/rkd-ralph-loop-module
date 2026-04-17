use sqlx::PgPool;
use chrono::Utc;
use crate::model::Iteration;
use crate::dto::IterationCreateResponse;
use crate::enumerator::Status;

pub async fn find_all_by_task_id(pool: &PgPool, task_id: i32) -> Result<Vec<Iteration>, sqlx::Error> {
    sqlx::query_as::<_, Iteration>(
        "SELECT id, created_date, last_modified_date, task_id
         FROM iteration
         WHERE task_id = $1 AND status = $2
         ORDER BY id",
    )
    .bind(task_id)
    .bind(Status::A)
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<Iteration>, sqlx::Error> {
    sqlx::query_as::<_, Iteration>(
        "SELECT id, created_date, last_modified_date, task_id
         FROM iteration
         WHERE id = $1 AND status = $2",
    )
    .bind(id)
    .bind(Status::A)
    .fetch_optional(pool)
    .await
}

pub async fn insert(pool: &PgPool, task_id: i32) -> Result<IterationCreateResponse, sqlx::Error> {
    let now = Utc::now().naive_utc();
    sqlx::query_as::<_, IterationCreateResponse>(
        "INSERT INTO iteration (created_date, last_modified_date, status, task_id)
         VALUES ($1, $2, $3, $4)
         RETURNING id, created_date, task_id",
    )
    .bind(now)
    .bind(now)
    .bind(Status::A)
    .bind(task_id)
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, id: i32, task_id: i32) -> Result<Option<Iteration>, sqlx::Error> {
    let now = Utc::now().naive_utc();
    sqlx::query_as::<_, Iteration>(
        "UPDATE iteration SET last_modified_date = $1, task_id = $2
         WHERE id = $3 AND status = $4
         RETURNING id, created_date, last_modified_date, task_id",
    )
    .bind(now)
    .bind(task_id)
    .bind(id)
    .bind(Status::A)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let now = Utc::now().naive_utc();
    let result = sqlx::query(
        "UPDATE iteration SET status = $1, last_modified_date = $2 WHERE id = $3 AND status = $4",
    )
    .bind(Status::I)
    .bind(now)
    .bind(id)
    .bind(Status::A)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}