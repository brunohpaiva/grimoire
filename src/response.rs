use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

#[derive(Debug)]
pub enum AppError {
    BadRequest,
    NotFound,
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Template)]
        #[template(path = "error.html")]
        struct ErrorTemplate {
            code: u16,
            message: String,
        }

        let (status, message) = match self {
            AppError::BadRequest => (StatusCode::BAD_REQUEST, "Bad Request".to_owned()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not Found".to_owned()),
            AppError::Internal(err) => {
                tracing::error!(%err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
        };

        (
            status,
            HtmlTemplate(ErrorTemplate {
                code: status.as_u16(),
                message,
            }),
        )
            .into_response()
    }
}

pub struct HtmlTemplate<T: Template>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
        }
    }
}
