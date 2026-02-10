//! Handler generation macros
//!
//! Eliminates repetitive handler boilerplate for the five common patterns
//! found across the service gateway. Each macro generates a thin `async fn`
//! that delegates to a service method.
//!
//! # Required imports at call site
//!
//! All macros assume the standard handler imports are in scope:
//! `Arc`, `State`, `Json`, `AppJson`, `ServerError`, `AppState`.
//! Path macros additionally need `Path`; created macros need `StatusCode`.

/// State + JSON body -> `Result<Json<Resp>, ServerError>`
///
/// Service signature: `fn(&Req, &AppState) -> Result<Resp, ServerError>`
macro_rules! json_handler {
    (
        $(#[$meta:meta])*
        $vis:vis async fn $name:ident($req:ty => $resp:ty) = $service:expr;
    ) => {
        $(#[$meta])*
        $vis async fn $name(
            State(state): State<Arc<AppState>>,
            AppJson(request): AppJson<$req>,
        ) -> Result<Json<$resp>, ServerError> {
            let response = $service(&request, &state)?;
            Ok(Json(response))
        }
    };
}

/// State only -> `Result<Json<Resp>, ServerError>`
///
/// Service signature: `fn(&AppState) -> Result<Resp, ServerError>`
macro_rules! get_handler {
    (
        $(#[$meta:meta])*
        $vis:vis async fn $name:ident(=> $resp:ty) = $service:expr;
    ) => {
        $(#[$meta])*
        $vis async fn $name(
            State(state): State<Arc<AppState>>,
        ) -> Result<Json<$resp>, ServerError> {
            let response = $service(&state)?;
            Ok(Json(response))
        }
    };
}

/// JSON body only (no state) -> `Result<Json<Resp>, ServerError>`
///
/// Service signature: `fn(&Req) -> Result<Resp, ServerError>`
macro_rules! stateless_json_handler {
    (
        $(#[$meta:meta])*
        $vis:vis async fn $name:ident($req:ty => $resp:ty) = $service:expr;
    ) => {
        $(#[$meta])*
        $vis async fn $name(
            AppJson(request): AppJson<$req>,
        ) -> Result<Json<$resp>, ServerError> {
            let response = $service(&request)?;
            Ok(Json(response))
        }
    };
}

/// State + Path -> `Result<Json<Resp>, ServerError>`
///
/// Service signature: `fn(&str, &AppState) -> Result<Resp, ServerError>`
macro_rules! path_handler {
    (
        $(#[$meta:meta])*
        $vis:vis async fn $name:ident(=> $resp:ty) = $service:expr;
    ) => {
        $(#[$meta])*
        $vis async fn $name(
            State(state): State<Arc<AppState>>,
            Path(id): Path<String>,
        ) -> Result<Json<$resp>, ServerError> {
            let response = $service(&id, &state)?;
            Ok(Json(response))
        }
    };
}

/// State + Path + JSON body -> `Result<Json<Resp>, ServerError>`
///
/// Service signature: `fn(&str, &Req, &AppState) -> Result<Resp, ServerError>`
macro_rules! path_json_handler {
    (
        $(#[$meta:meta])*
        $vis:vis async fn $name:ident($req:ty => $resp:ty) = $service:expr;
    ) => {
        $(#[$meta])*
        $vis async fn $name(
            State(state): State<Arc<AppState>>,
            Path(id): Path<String>,
            AppJson(request): AppJson<$req>,
        ) -> Result<Json<$resp>, ServerError> {
            let response = $service(&id, &request, &state)?;
            Ok(Json(response))
        }
    };
}

/// State + JSON body -> `Result<(StatusCode::CREATED, Json<Resp>), ServerError>`
///
/// Service signature: `fn(&Req, &AppState) -> Result<Resp, ServerError>`
macro_rules! json_created_handler {
    (
        $(#[$meta:meta])*
        $vis:vis async fn $name:ident($req:ty => $resp:ty) = $service:expr;
    ) => {
        $(#[$meta])*
        $vis async fn $name(
            State(state): State<Arc<AppState>>,
            AppJson(request): AppJson<$req>,
        ) -> Result<(StatusCode, Json<$resp>), ServerError> {
            let response = $service(&request, &state)?;
            Ok((StatusCode::CREATED, Json(response)))
        }
    };
}
