use std::collections::HashMap;
use spin_sdk::http::Response;

// ── Query string ──────────────────────────────────────────────────────────

/// Decodifica um componente percent-encoded da URL.
/// `+` → espaço, `%XX` → byte correspondente.
pub fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    // Reassemble UTF-8 multi-byte sequences
                    out.push(byte as char);
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

/// Parseia `key=value&key2=value2` → HashMap.
pub fn parse_qs(query: &str) -> HashMap<String, String> {
    if query.is_empty() {
        return HashMap::new();
    }
    query
        .split('&')
        .filter_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let k = it.next()?;
            let v = it.next().unwrap_or("");
            Some((url_decode(k), url_decode(v)))
        })
        .collect()
}

/// Parseia f64 de um parâmetro, retornando erro amigável se ausente/inválido.
pub fn require_f64(
    qs: &HashMap<String, String>,
    key: &str,
) -> anyhow::Result<f64> {
    let raw = qs
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("parâmetro '{}' ausente", key))?;
    raw.parse::<f64>()
        .map_err(|_| anyhow::anyhow!("parâmetro '{}' inválido: '{}'", key, raw))
}

/// Parseia String obrigatória de um parâmetro.
pub fn require_str<'a>(
    qs: &'a HashMap<String, String>,
    key: &str,
) -> anyhow::Result<&'a str> {
    qs.get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("parâmetro '{}' ausente", key))
}

// ── Response builders ─────────────────────────────────────────────────────

const CORS_ORIGIN:  &str = "*";
const CORS_METHODS: &str = "GET, OPTIONS";
const CORS_HEADERS: &str = "Content-Type";

/// Resposta JSON com headers CORS e cache de 5 minutos.
pub fn json_ok(body: String) -> Response {
    Response::builder()
        .status(200)
        .header("content-type", "application/json; charset=utf-8")
        .header("access-control-allow-origin",  CORS_ORIGIN)
        .header("access-control-allow-methods", CORS_METHODS)
        .header("access-control-allow-headers", CORS_HEADERS)
        .header("cache-control", "public, max-age=300")
        .body(body)
        .build()
}

/// Resposta de erro JSON.
pub fn error_json(status: u16, message: &str) -> Response {
    let body = format!(
        r#"{{"error":true,"message":{}}}"#,
        serde_json::to_string(message).unwrap_or_else(|_| format!("\"{}\"", message))
    );
    Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .header("access-control-allow-origin", CORS_ORIGIN)
        .body(body)
        .build()
}

/// Resposta para CORS preflight (OPTIONS).
pub fn cors_preflight() -> Response {
    Response::builder()
        .status(204)
        .header("access-control-allow-origin",  CORS_ORIGIN)
        .header("access-control-allow-methods", CORS_METHODS)
        .header("access-control-allow-headers", CORS_HEADERS)
        .header("access-control-max-age", "86400")
        .body("")
        .build()
}

/// Health check response.
pub fn health() -> Response {
    json_ok(
        r#"{"status":"ok","region":"Alagoas, BR","version":"0.1.0","endpoints":["/geocode","/reverse","/route","/health"]}"#
            .to_string(),
    )
}

/// 404 com path informado.
pub fn not_found(path: &str) -> Response {
    error_json(
        404,
        &format!("rota '{}' não encontrada. Endpoints: /geocode /reverse /route /health", path),
    )
}