use sqlx::PgPool;
use chrono::Utc;
use crate::model::Task;
use crate::dto::TaskCreateResponse;
use crate::enumerator::Status;

pub async fn find_all_by_use_case_id(pool: &PgPool, use_case_id: i32) -> Result<Vec<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(
        "SELECT id, name, sequence, type, path, prompt, created_date, last_modified_date, use_case_id
         FROM task
         WHERE use_case_id = $1 AND status = $2
         ORDER BY sequence",
    )
    .bind(use_case_id)
    .bind(Status::A)
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(
        "SELECT id, name, sequence, type, path, prompt, created_date, last_modified_date, use_case_id
         FROM task
         WHERE id = $1 AND status = $2",
    )
    .bind(id)
    .bind(Status::A)
    .fetch_optional(pool)
    .await
}

pub async fn insert(
    pool: &PgPool,
    name: String,
    task_type: String,
    path: String,
    prompt: String,
    use_case_id: i32,
) -> Result<TaskCreateResponse, sqlx::Error> {

    // 🔢 calcula próximo sequence
    let next_sequence: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sequence), 0) + 1
         FROM task
         WHERE use_case_id = $1 AND status = $2"
    )
    .bind(use_case_id)
    .bind(Status::A)
    .fetch_one(pool)
    .await?;

    let now = Utc::now().naive_utc();

    sqlx::query_as::<_, TaskCreateResponse>(
        "INSERT INTO task (name, sequence, type, path, prompt, created_date, last_modified_date, status, use_case_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING id, name, sequence, type, path, prompt, created_date, use_case_id",
    )
    .bind(name)
    .bind(next_sequence)
    .bind(task_type)
    .bind(path)
    .bind(prompt)
    .bind(now)
    .bind(now)
    .bind(Status::A)
    .bind(use_case_id)
    .fetch_one(pool)
    .await
}

pub async fn update(
    pool: &PgPool,
    id: i32,
    name: String,
    task_type: String,
    path: String,
    prompt: String,
    use_case_id: i32,
) -> Result<Option<Task>, sqlx::Error> {
    let now = Utc::now().naive_utc();

    sqlx::query_as::<_, Task>(
        "UPDATE task SET name = $1, type = $2, path = $3, prompt = $4,
         last_modified_date = $5, use_case_id = $6
         WHERE id = $7 AND status = $8
         RETURNING id, name, sequence, type, path, prompt, created_date, last_modified_date, use_case_id",
    )
    .bind(name)
    .bind(task_type)
    .bind(path)
    .bind(prompt)
    .bind(now)
    .bind(use_case_id)
    .bind(id)
    .bind(Status::A)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let now = Utc::now().naive_utc();
    let result = sqlx::query(
        "UPDATE task SET status = $1, last_modified_date = $2 WHERE id = $3 AND status = $4",
    )
    .bind(Status::I)
    .bind(now)
    .bind(id)
    .bind(Status::A)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// 🔍 Verifica se existe alguma task ativa para um use_case
pub async fn exists_by_use_case_id(
    pool: &PgPool,
    use_case_id: i32,
) -> Result<bool, sqlx::Error> {
    let exists: Option<i32> = sqlx::query_scalar(
        "SELECT 1
         FROM task
         WHERE use_case_id = $1 AND status = $2
         LIMIT 1"
    )
    .bind(use_case_id)
    .bind(Status::A)
    .fetch_optional(pool)
    .await?;

    Ok(exists.is_some())
}