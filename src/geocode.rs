/// GET /geocode?q=<query>[&limit=<1-10>]
///
/// Proxy para o Nominatim OSM com `viewbox` fixado em Alagoas.
/// Retorna array compatível com Nominatim jsonv2.
use anyhow::Context;
use spin_sdk::http::{send, Method, Request, Response};

use crate::bounds;
use crate::errors::ApiError;
use crate::helpers::{json_ok, parse_qs, require_str};

/// User-Agent exigido pela política de uso do Nominatim.
const USER_AGENT: &str = "cardapio-geo-api/0.1 (https://github.com/seu-usuario/cardapio-geo-api)";

pub async fn handle(query: &str) -> Result<Response, ApiError> {
    let qs = parse_qs(query);

    // ── Parâmetros ────────────────────────────────────────────────────────
    let q = match require_str(&qs, "q") {
        Ok(v) if !v.trim().is_empty() => v.to_string(),
        _ => return Err(ApiError::bad_request("parâmetro 'q' ausente ou vazio")),
    };

    let limit: u8 = qs
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
        .clamp(1, 10);

    // ── Monta URL do Nominatim ────────────────────────────────────────────
    let viewbox = bounds::ALAGOAS.as_viewbox();
    let url = format!(
        "https://nominatim.openstreetmap.org/search\
         ?q={q}\
         &format=jsonv2\
         &addressdetails=1\
         &limit={limit}\
         &countrycodes=br\
         &viewbox={viewbox}\
         &bounded=0\
         &accept-language=pt-BR,pt,en",
        q = urlencode(&q),
        limit = limit,
        viewbox = urlencode(&viewbox),
    );

    // ── Chama Nominatim ───────────────────────────────────────────────────
    let upstream_req = Request::builder()
        .method(Method::Get)
        .uri(&url)
        .header("user-agent", USER_AGENT)
        .header("accept", "application/json")
        .body("")
        .build();

    let upstream_resp: spin_sdk::http::Response = send(upstream_req)
        .await
        .context("falha ao contactar Nominatim")
        .map_err(|e| ApiError::bad_gateway("nominatim", e.to_string()))?;

    if *upstream_resp.status() != 200 {
        return Err(ApiError::bad_gateway(
            "nominatim",
            format!("status HTTP {}", upstream_resp.status()),
        ));
    }

    // ── Parseia e enriquece resultados ────────────────────────────────────
    let raw = std::str::from_utf8(upstream_resp.body())
        .context("resposta do Nominatim não é UTF-8")
        .map_err(ApiError::from)?;

    let mut results: Vec<serde_json::Value> = serde_json::from_str(raw)
        .context("resposta do Nominatim inválida")
        .map_err(ApiError::from)?;

    for item in &mut results {
        // Nota: anotações explícitas necessárias — o compilador não infere o
        // tipo de `v` em `and_then` quando o contexto externo ainda é ambíguo.
        if let (Some(lat_str), Some(lon_str)) = (
            item.get("lat").and_then(|v: &serde_json::Value| v.as_str()),
            item.get("lon").and_then(|v: &serde_json::Value| v.as_str()),
        ) {
            if let (Ok(lat), Ok(lon)) = (lat_str.parse::<f64>(), lon_str.parse::<f64>()) {
                let (in_al, in_ext) = bounds::classify(lat, lon);
                item["within_alagoas"] = serde_json::Value::Bool(in_al);
                item["within_region"] = serde_json::Value::Bool(in_ext);
            }
        }
    }

    let body = serde_json::to_string(&results).map_err(anyhow::Error::from)?;
    Ok(json_ok(body))
}

/// Percent-encode mínimo para montar URLs de query string.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | ',' => out.push(c),
            ' ' => out.push('+'),
            c => {
                let mut buf = [0u8; 4];
                for b in c.encode_utf8(&mut buf).bytes() {
                    out.push('%');
                    out.push_str(&format!("{:02X}", b));
                }
            }
        }
    }
    out
}
