use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::health_check,
    ),
    components(
        schemas(
            crate::error::ErrorResponse,
        )
    ),
    tags(
        (name = "televent", description = "Televent API")
    ),
    modifiers(&crate::add_security_scheme)
)]
pub struct ApiDoc;
