pub mod iteration;
pub mod project;
pub mod task;
pub mod use_case;

use axum::Router;
use axum::routing::get;
use sqlx::PgPool;

pub fn routes(pool: PgPool) -> Router {
    Router::new()
        // Projects
        .route("/projects",       get(project::get_projects).post(project::create_project))
        .route("/projects/:id",   get(project::get_project).put(project::update_project).delete(project::delete_project))

        // Use Cases
        .route("/projects/:project_id/use-cases", get(use_case::get_use_cases_by_project))
        .route("/use-cases",      axum::routing::post(use_case::create_use_case))
        .route("/use-cases/:id",  get(use_case::get_use_case).put(use_case::update_use_case).delete(use_case::delete_use_case))

        // Tasks
        .route("/use-cases/:use_case_id/tasks", get(task::get_tasks_by_use_case))
        .route("/tasks",          axum::routing::post(task::create_task))
        .route("/tasks/:id",      get(task::get_task).put(task::update_task).delete(task::delete_task))

        // Iterations
        .route("/tasks/:task_id/iterations", get(iteration::get_iterations_by_task))
        .route("/iterations",     axum::routing::post(iteration::create_iteration))
        .route("/iterations/:id", get(iteration::get_iteration).put(iteration::update_iteration).delete(iteration::delete_iteration))

        .with_state(pool)
}