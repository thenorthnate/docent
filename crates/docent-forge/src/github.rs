use octocrab::Octocrab;

use crate::{ConnectError, Host, Token};

#[derive(serde::Deserialize)]
struct User {
    login: String,
}

/// Fetches the login of the user `token` belongs to, with `GET /user`.
pub(crate) async fn current_login(
    host: &Host,
    api_base: &str,
    token: &Token,
) -> Result<String, ConnectError> {
    let unreachable = |reason: String| ConnectError::Unreachable {
        host: host.as_str().to_string(),
        reason,
    };
    let client = Octocrab::builder()
        .base_uri(api_base)
        .map_err(|err| unreachable(err.to_string()))?
        .personal_token(token.expose().to_string())
        .build()
        .map_err(|err| unreachable(err.to_string()))?;

    match client.get::<User, _, ()>("/user", None).await {
        Ok(user) => Ok(user.login),
        // GitHub errors carry the response body, which the server controls,
        // so only the status is passed on.
        Err(octocrab::Error::GitHub { source, .. }) => match source.status_code.as_u16() {
            401 => Err(ConnectError::TokenRejected(host.as_str().to_string())),
            status => Err(ConnectError::HttpStatus {
                host: host.as_str().to_string(),
                status,
            }),
        },
        Err(err) => Err(unreachable(err.to_string())),
    }
}
