use sqlx::PgPool;
use chrono::Utc;
use crate::model::Project;
use crate::enumerator::Status;

pub async fn find_all(pool: &PgPool) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>("SELECT id, name, created_date, last_modified_date FROM project WHERE status = $1 ORDER BY id")
        .bind(Status::A)
        .fetch_all(pool)
        .await
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>("SELECT id, name, created_date, last_modified_date FROM project WHERE id = $1 AND status = $2")
        .bind(id)
        .bind(Status::A)
        .fetch_optional(pool)
        .await
}

pub async fn insert(pool: &PgPool, name: String) -> Result<Project, sqlx::Error> {
    let now = Utc::now().naive_utc();
    sqlx::query_as::<_, Project>(
        "INSERT INTO project (name, created_date, last_modified_date, status)
         VALUES ($1, $2, $3, $4)
         RETURNING id, name, created_date, last_modified_date",
    )
    .bind(name)
    .bind(now)
    .bind(now)
    .bind(Status::A)
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, id: i32, name: String) -> Result<Option<Project>, sqlx::Error> {
    let now = Utc::now().naive_utc();
    sqlx::query_as::<_, Project>(
        "UPDATE project SET name = $1, last_modified_date = $2
         WHERE id = $3 AND status = $4
         RETURNING id, name, created_date, last_modified_date",
    )
    .bind(name)
    .bind(now)
    .bind(id)
    .bind(Status::A)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let now = Utc::now().naive_utc();
    let result = sqlx::query(
        "UPDATE project SET status = $1, last_modified_date = $2 WHERE id = $3 AND status = $4",
    )
    .bind(Status::I)
    .bind(now)
    .bind(id)
    .bind(Status::A)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}