use spin_sdk::http::{IntoResponse, Method, Request};

mod bounds;
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

    // ── Parseia path e query da URI ───────────────────────────────────────
    // req.uri() pode retornar a URL completa (http://host/path?query)
    // ou só o path (/path?query) dependendo do cliente.
    // Isolamos sempre só o path + query removendo scheme://host se presentes.
    let uri = req.uri();

    // Remove scheme://host se existir: "http://127.0.0.1:3000/route?x=1" → "/route?x=1"
    let path_and_query = if let Some(after_scheme) = uri.find("://") {
        // pula "://" e avança até a próxima '/' (início do path)
        let after_host = &uri[after_scheme + 3..];
        match after_host.find('/') {
            Some(i) => &after_host[i..],
            None    => "/",
        }
    } else {
        uri  // já é só path
    };

    // Separa path de query string pelo primeiro '?'
    let (path, query) = match path_and_query.find('?') {
        Some(i) => (&path_and_query[..i], &path_and_query[i + 1..]),
        None    => (path_and_query, ""),
    };

    // ── Roteamento ────────────────────────────────────────────────────────
    let result = match path {
        "/geocode" => geocode::handle(query).await,
        "/reverse" => reverse::handle(query).await,
        "/route"   => route::handle(query).await,
        "/health"  => Ok(helpers::health()),
        other      => Ok(helpers::not_found(other)),
    };

    match result {
        Ok(resp) => Ok(resp),
        Err(err) => {
            eprintln!("[cardapio-geo-api] erro interno: {:#}", err);
            Ok(helpers::error_json(500, &err.to_string()))
        }
    }
}