use std::net::{Ipv4Addr, SocketAddr};

use scalar_warrper::{Scalar, Servable};
use std::io::Error;
use tokio::net::TcpListener;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};
use utoipa_axum::router::OpenApiRouter;

const TODO_TAG: &str = "todo";

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1/todos", todo::router())
        .split_for_parts();

    let router = router.merge(Scalar::with_url("/scalar", api));

    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, 8080));
    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, router.into_make_service()).await
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    tags(
            (name = TODO_TAG, description = "Todo items management API")
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("todo_apikey"))),
            )
        }
    }
}

mod todo {
    use std::sync::Arc;

    use axum::{
        Json,
        extract::{Path, Query, State},
        response::IntoResponse,
    };
    use http::{HeaderMap, StatusCode};
    use serde::{Deserialize, Serialize};
    use tokio::sync::Mutex;
    use utoipa::{IntoParams, ToSchema};
    use utoipa_axum::{router::OpenApiRouter, routes};

    use crate::TODO_TAG;

    /// 将待办事项信息保存在内存中
    type Store = Mutex<Vec<Todo>>;

    /// 待办事项
    #[derive(Serialize, Deserialize, ToSchema, Clone)]
    struct Todo {
        /// 唯一标识
        id: i32,
        /// 描述
        #[schema(example = "Buy groceries")]
        value: String,
        /// 是否完成
        done: bool,
    }

    /// 错误枚举
    #[derive(Serialize, Deserialize, ToSchema)]
    enum TodoError {
        /// 待办事项已经存在，冲突
        #[schema(example = "Todo already exists")]
        Conflict(String),
        /// 通过唯一标识没有找到待办事项
        #[schema(example = "The task is not found by id = 1")]
        NotFound(String),
        /// 未授权操作
        #[schema(example = "Missing api key")]
        Unauthorized(String),
    }

    pub(super) fn router() -> OpenApiRouter {
        let store = Arc::new(Store::default());
        OpenApiRouter::new()
            .routes(routes!(list_todos, create_todo))
            .routes(routes!(search_todos))
            .routes(routes!(mark_done, delete_todo))
            .with_state(store)
    }

    /// 获取全部的待办事项
    ///
    /// 从内存中获取全部的待办事项
    #[utoipa::path(
        get,
        path = "",
        tag = TODO_TAG,
        responses(
            (status = 200, description = "List all todos successfully", body = [Todo])
        )
    )]
    async fn list_todos(State(store): State<Arc<Store>>) -> Json<Vec<Todo>> {
        let todos = store.lock().await.clone();

        Json(todos)
    }

    /// 查询待办事项的选贤
    #[derive(Deserialize, IntoParams)]
    struct TodoSearchQuery {
        /// 按值搜索，区分大小写
        value: String,
        /// 按状态搜索，是否完成
        done: bool,
    }

    /// 通过查询选项搜索待办事项
    ///
    /// 通过查询参数搜索待办事项，并返回匹配的待办事项
    #[utoipa::path(
        get,
        path = "/search",
        tag = TODO_TAG,
        params(
            TodoSearchQuery
        ),
        responses(
            (status = 200, description = "List matching todos by query", body = [Todo])
        )
    )]
    async fn search_todos(
        State(store): State<Arc<Store>>,
        query: Query<TodoSearchQuery>,
    ) -> Json<Vec<Todo>> {
        Json(
            store
                .lock()
                .await
                .iter()
                .filter(|todo| {
                    todo.value.to_lowercase() == query.value.to_lowercase()
                        && todo.done == query.done
                })
                .cloned()
                .collect(),
        )
    }

    /// 创建一个新的待办事项
    ///
    /// 尝试在内存存储中创建新待办事项，如果该事项已经存在，则以409冲突失败
    #[utoipa::path(
        post,
        path = "",
        tag = TODO_TAG,
        responses(
            (status = 201, description = "Todo item created successfully", body = Todo),
            (status = 409, description = "Todo already exists", body = TodoError)
        )
    )]
    async fn create_todo(
        State(store): State<Arc<Store>>,
        Json(todo): Json<Todo>,
    ) -> impl IntoResponse {
        let mut todos = store.lock().await;

        todos
            .iter_mut()
            .find(|existing_todo| existing_todo.id == todo.id)
            .map(|found| {
                (
                    StatusCode::CONFLICT,
                    Json(TodoError::Conflict(format!(
                        "todo already exists: {}",
                        found.id
                    ))),
                )
                    .into_response()
            })
            .unwrap_or_else(|| {
                todos.push(todo.clone());

                (StatusCode::CREATED, Json(todo)).into_response()
            })
    }

    /// 标记待办事项已经完成
    ///
    /// 通过给定 id, 标记待办事项已完成完成；如果成功，只返回状态200；如果找不到待办事项，则返回状态404
    #[utoipa::path(
        put,
        path = "/{id}",
        tag = TODO_TAG,
        responses(
            (status = 200, description = "Todo marked done successfully"),
            (status = 404, description = "Todo not found")
        ),
        params(
            ("id" = i32, Path, description = "Todo database id")
        ),
        security(
            (), // <-- make optional authentication
            ("api_key" = [])
        )
    )]
    async fn mark_done(
        Path(id): Path<i32>,
        State(store): State<Arc<Store>>,
        headers: HeaderMap,
    ) -> StatusCode {
        match check_api_key(false, headers) {
            Ok(_) => (),
            Err(_) => return StatusCode::UNAUTHORIZED,
        }

        let mut todos = store.lock().await;

        todos
            .iter_mut()
            .find(|todo| todo.id == id)
            .map(|todo| {
                todo.done = true;
                StatusCode::OK
            })
            .unwrap_or(StatusCode::NOT_FOUND)
    }

    /// 删除待办事项
    ///
    /// 按 id 从内存存储中删除待办事项。如果没有找到对应的待办，则返回 404；如果没有权限删除，则返回 401；删除成功，则返回 200
    #[utoipa::path(
        delete,
        path = "/{id}",
        tag = TODO_TAG,
        responses(
            (status = 200, description = "Todo marked done successfully"),
            (status = 401, description = "Unauthorized to delete Todo", body = TodoError, example = json!(TodoError::Unauthorized(String::from("missing api key")))),
            (status = 404, description = "Todo not found", body = TodoError, example = json!(TodoError::NotFound(String::from("id = 1"))))
        ),
        params(
            ("id" = i32, Path, description = "Todo database id")
        ),
        security(
            ("api_key" = [])
        )
    )]
    async fn delete_todo(
        Path(id): Path<i32>,
        State(store): State<Arc<Store>>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        match check_api_key(true, headers) {
            Ok(_) => (),
            Err(error) => return error.into_response(),
        }

        let mut todos = store.lock().await;

        let len = todos.len();

        todos.retain(|todo| todo.id != id);

        if todos.len() != len {
            StatusCode::OK.into_response()
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(TodoError::NotFound(format!("id = {id}"))),
            )
                .into_response()
        }
    }

    /// 创建一个检查 api 的中间函数
    /// 这个作为例子足够了
    fn check_api_key(
        require_api_key: bool,
        headers: HeaderMap,
    ) -> Result<(), (StatusCode, Json<TodoError>)> {
        match headers.get("todo_apikey") {
            Some(header) if header != "utoipa-rocks" => Err((
                StatusCode::UNAUTHORIZED,
                Json(TodoError::Unauthorized(String::from("incorrect api key"))),
            )),
            None if require_api_key => Err((
                StatusCode::UNAUTHORIZED,
                Json(TodoError::Unauthorized(String::from("missing api key"))),
            )),
            _ => Ok(()),
        }
    }
}
