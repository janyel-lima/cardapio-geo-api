/// GET /reverse?lat=<lat>&lng=<lng>
///
/// Reverse geocoding via Nominatim.
/// Resposta: formato Nominatim padrão + `within_alagoas` + `within_region`.
use anyhow::Context;
use spin_sdk::http::{send, Method, Request, Response};

use crate::bounds;
use crate::errors::ApiError;
use crate::helpers::{json_ok, parse_qs, require_f64};

const USER_AGENT: &str = "cardapio-geo-api/0.1 (https://github.com/seu-usuario/cardapio-geo-api)";

pub async fn handle(query: &str) -> Result<Response, ApiError> {
    let qs = parse_qs(query);

    // ── Valida parâmetros ─────────────────────────────────────────────────
    let lat = require_f64(&qs, "lat").map_err(|e| ApiError::bad_request(e.to_string()))?;
    let lng = require_f64(&qs, "lng").map_err(|e| ApiError::bad_request(e.to_string()))?;

    // Rejeita coordenadas fora da região operacional antes de chamar o upstream.
    let (in_al, in_ext) = bounds::classify(lat, lng);
    if !in_ext {
        return Err(ApiError::unprocessable(format!(
            "coordenadas ({lat}, {lng}) estão fora da região operacional"
        )));
    }

    // ── Chama Nominatim /reverse ──────────────────────────────────────────
    let url = format!(
        "https://nominatim.openstreetmap.org/reverse\
         ?lat={lat}\
         &lon={lng}\
         &format=jsonv2\
         &addressdetails=1\
         &accept-language=pt-BR,pt,en",
    );

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

    // ── Enriquece resposta ────────────────────────────────────────────────
    let raw = std::str::from_utf8(upstream_resp.body())
        .context("resposta do Nominatim não é UTF-8")
        .map_err(ApiError::from)?;

    let mut result: serde_json::Value = serde_json::from_str(raw)
        .context("resposta do Nominatim inválida")
        .map_err(ApiError::from)?;

    // Nominatim retorna {"error":"..."} para coordenadas sem resultado.
    if result.get("error").is_some() {
        return Err(ApiError::not_found(
            "endereço não encontrado para essas coordenadas",
        ));
    }

    result["within_alagoas"] = serde_json::Value::Bool(in_al);
    result["within_region"] = serde_json::Value::Bool(in_ext);

    let body = serde_json::to_string(&result).map_err(anyhow::Error::from)?;
    Ok(json_ok(body))
}
