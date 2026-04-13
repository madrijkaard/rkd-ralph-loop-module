use sqlx::PgPool;
use chrono::Utc;
use crate::model::UseCase;
use crate::enumerator::Status;

pub async fn find_all(pool: &PgPool) -> Result<Vec<UseCase>, sqlx::Error> {
    sqlx::query_as::<_, UseCase>("SELECT id, name, prompt, created_date, last_modified_date, project_id FROM use_case WHERE status = $1 ORDER BY id")
        .bind(Status::A)
        .fetch_all(pool)
        .await
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<UseCase>, sqlx::Error> {
    sqlx::query_as::<_, UseCase>("SELECT id, name, prompt, created_date, last_modified_date, project_id FROM use_case WHERE id = $1 AND status = $2")
        .bind(id)
        .bind(Status::A)
        .fetch_optional(pool)
        .await
}

pub async fn insert(
    pool: &PgPool,
    name: String,
    prompt: String,
    project_id: i32,
) -> Result<UseCase, sqlx::Error> {
    let now = Utc::now().naive_utc();
    sqlx::query_as::<_, UseCase>(
        "INSERT INTO use_case (name, prompt, created_date, last_modified_date, status, project_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, name, prompt, created_date, last_modified_date, project_id",
    )
    .bind(name)
    .bind(prompt)
    .bind(now)
    .bind(now)
    .bind(Status::A)
    .bind(project_id)
    .fetch_one(pool)
    .await
}

pub async fn update(
    pool: &PgPool,
    id: i32,
    name: String,
    prompt: String,
    project_id: i32,
) -> Result<Option<UseCase>, sqlx::Error> {
    let now = Utc::now().naive_utc();
    sqlx::query_as::<_, UseCase>(
        "UPDATE use_case SET name = $1, prompt = $2, last_modified_date = $3, project_id = $4
         WHERE id = $5 AND status = $6
         RETURNING id, name, prompt, created_date, last_modified_date, project_id",
    )
    .bind(name)
    .bind(prompt)
    .bind(now)
    .bind(project_id)
    .bind(id)
    .bind(Status::A)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let now = Utc::now().naive_utc();
    let result = sqlx::query(
        "UPDATE use_case SET status = $1, last_modified_date = $2 WHERE id = $3 AND status = $4",
    )
    .bind(Status::I)
    .bind(now)
    .bind(id)
    .bind(Status::A)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}