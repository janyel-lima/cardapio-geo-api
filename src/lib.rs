use spin_sdk::http::{IntoResponse, Method, Request, Response};

mod bounds;
mod errors;
mod geocode;
mod helpers;
mod reverse;
mod route;

#[spin_sdk::http_component]
async fn handle(req: Request) -> anyhow::Result<impl IntoResponse> {
    // ── CORS preflight ────────────────────────────────────────────────────
    if req.method() == &Method::Options {
        return Ok(helpers::cors_preflight());
    }

    // ── Só aceita GET ─────────────────────────────────────────────────────
    if req.method() != &Method::Get {
        return Ok(helpers::error_json(405, "método não permitido — use GET"));
    }

    // ── Isola path + query da URI ─────────────────────────────────────────
    // req.uri() pode vir como URL completa (http://host/path?q) ou só path.
    let uri = req.uri();
    let path_and_query = if let Some(after_scheme) = uri.find("://") {
        let after_host = &uri[after_scheme + 3..];
        match after_host.find('/') {
            Some(i) => &after_host[i..],
            None => "/",
        }
    } else {
        uri
    };

    let (path, query) = match path_and_query.find('?') {
        Some(i) => (&path_and_query[..i], &path_and_query[i + 1..]),
        None => (path_and_query, ""),
    };

    // ── Roteamento ────────────────────────────────────────────────────────
    // Handlers retornam Result<Response, ApiError>.
    // Erros de servidor são logados; erros de cliente (4xx) são silenciosos.
    let result: Result<Response, errors::ApiError> = match path {
        "/geocode" => geocode::handle(query).await,
        "/reverse" => reverse::handle(query).await,
        "/route" => route::handle(query).await,
        "/health" => return Ok(helpers::health()),
        other => return Ok(helpers::not_found(other)),
    };

    Ok(match result {
        Ok(resp) => resp,
        Err(e) => {
            if e.is_server_error() {
                eprintln!("[geo-api] erro interno em {path}: {e:#}");
            }
            Response::from(e)
        }
    })
}