/// GET /reverse?lat=<lat>&lng=<lng>
///
/// Reverse geocoding via Nominatim.
/// Resposta é o formato Nominatim padrão + campos extras:
///   - `within_alagoas`  {bool}
///   - `within_region`   {bool}
///
/// Compatível com qualquer código que leia a resposta do Nominatim /reverse.

use anyhow::Context;
use spin_sdk::http::{Method, Request, Response, send};

use crate::bounds;
use crate::helpers::{error_json, json_ok, parse_qs, require_f64};

const USER_AGENT: &str =
    "cardapio-geo-api/0.1 (https://github.com/seu-usuario/cardapio-geo-api)";

pub async fn handle(query: &str) -> anyhow::Result<Response> {
    let qs = parse_qs(query);

    // ── Valida parâmetros ─────────────────────────────────────────────────
    let lat = match require_f64(&qs, "lat") {
        Ok(v) => v,
        Err(e) => return Ok(error_json(400, &e.to_string())),
    };
    let lng = match require_f64(&qs, "lng") {
        Ok(v) => v,
        Err(e) => return Ok(error_json(400, &e.to_string())),
    };

    // Rejeita coordenadas obviamente fora da região
    let (in_al, in_ext) = bounds::classify(lat, lng);
    if !in_ext {
        return Ok(error_json(
            422,
            &format!(
                "coordenadas ({}, {}) estão fora da região operacional (Alagoas e limítrofes)",
                lat, lng
            ),
        ));
    }

    // ── Chama Nominatim /reverse ──────────────────────────────────────────
    let url = format!(
        "https://nominatim.openstreetmap.org/reverse\
         ?lat={lat}\
         &lon={lng}\
         &format=jsonv2\
         &addressdetails=1\
         &accept-language=pt-BR,pt,en",
        lat = lat,
        lng = lng,
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
        .context("falha ao contactar Nominatim")?;

    if *upstream_resp.status() != 200 {
        return Ok(error_json(
            502,
            &format!("Nominatim retornou status {}", upstream_resp.status()),
        ));
    }

    // ── Enriquece resposta ────────────────────────────────────────────────
    let raw = std::str::from_utf8(upstream_resp.body())
        .context("resposta do Nominatim não é UTF-8")?;

    let mut result: serde_json::Value =
        serde_json::from_str(raw).context("resposta do Nominatim inválida")?;

    // Nominatim retorna `{"error":"..."}` se não encontrar nada
    if result.get("error").is_some() {
        return Ok(error_json(404, "endereço não encontrado para essas coordenadas"));
    }

    result["within_alagoas"] = serde_json::Value::Bool(in_al);
    result["within_region"]  = serde_json::Value::Bool(in_ext);

    Ok(json_ok(serde_json::to_string(&result)?))
}