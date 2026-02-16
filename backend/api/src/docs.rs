use utoipa::OpenApi;
use crate::SecurityAddon;

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
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;
