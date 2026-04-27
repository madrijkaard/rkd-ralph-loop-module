pub mod project;
pub mod use_case;
pub mod task;
pub mod task_type;
pub mod iteration;
pub mod engine;

use axum::{Router, routing::{get, post}};
use sqlx::PgPool;

use crate::config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub settings: Settings,
}

pub fn routes(state: AppState) -> Router {

    Router::new()
        .route("/projects", get(project::get_projects).post(project::create_project))
        .route("/projects/:id", get(project::get_project).put(project::update_project).delete(project::delete_project),)
        .route("/projects/:project_id/use-cases", get(use_case::get_use_cases_by_project))
        .route("/use-cases", post(use_case::create_use_case))
        .route("/use-cases/:id", get(use_case::get_use_case).put(use_case::update_use_case).delete(use_case::delete_use_case),)
        .route("/use-cases/:use_case_id/tasks", get(task::get_tasks_by_use_case))
        .route("/tasks", post(task::create_task))
        .route("/tasks/:id", get(task::get_task).put(task::update_task).delete(task::delete_task),)
        .route("/tasks/:id/execute", post(task::execute_task))
        .route("/tasks/:task_id/iterations", get(iteration::get_iterations_by_task))
        .route("/iterations", post(iteration::create_iteration))
        .route("/iterations/:id", get(iteration::get_iteration).put(iteration::update_iteration).delete(iteration::delete_iteration),)
        .route("/task-types", get(task_type::get_task_types))
        .route("/engine/models", get(engine::get_models))
        .with_state(state)
}