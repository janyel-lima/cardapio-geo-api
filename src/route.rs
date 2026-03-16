/// GET /route?from_lat=<f>&from_lng=<f>&to_lat=<f>&to_lng=<f>
///
/// Cálculo de rota via OSRM.
/// A URL base é lida da variável Spin `osrm_base` (configurável em runtime).

use anyhow::Context;
use spin_sdk::http::{Method, Request, Response, send};
use spin_sdk::variables;

use crate::bounds;
use crate::errors::ApiError;
use crate::helpers::{json_ok, parse_qs, require_f64};

pub async fn handle(query: &str) -> Result<Response, ApiError> {
    let qs = parse_qs(query);

    // ── Lê URL base do OSRM ───────────────────────────────────────────────
    let osrm_base = variables::get("osrm_base")
        .context("variável 'osrm_base' não definida no spin.toml")
        .map_err(ApiError::from)?;

    // ── Valida parâmetros ─────────────────────────────────────────────────
    let from_lat = require_f64(&qs, "from_lat").map_err(|e| ApiError::bad_request(e.to_string()))?;
    let from_lng = require_f64(&qs, "from_lng").map_err(|e| ApiError::bad_request(e.to_string()))?;
    let to_lat   = require_f64(&qs, "to_lat").map_err(|e| ApiError::bad_request(e.to_string()))?;
    let to_lng   = require_f64(&qs, "to_lng").map_err(|e| ApiError::bad_request(e.to_string()))?;

    // ── Valida região ─────────────────────────────────────────────────────
    let (_, from_ext) = bounds::classify(from_lat, from_lng);
    let (to_al, to_ext) = bounds::classify(to_lat, to_lng);

    if !from_ext {
        return Err(ApiError::unprocessable(
            "origem está fora da região operacional (Alagoas e limítrofes)",
        ));
    }

    // Destino fora da área → resposta semântica 200 com out_of_range=true.
    // Não é um erro HTTP — é uma resposta de negócio esperada pelo frontend.
    if !to_ext {
        let body = serde_json::json!({
            "ok":             false,
            "out_of_range":   true,
            "within_alagoas": false,
            "within_region":  false,
            "distance_km":    null,
            "duration_min":   null,
            "routes":         [],
            "message":        "destino fora da área de entrega"
        });
        return Ok(json_ok(serde_json::to_string(&body).unwrap()));
    }

    // ── Monta URL OSRM ────────────────────────────────────────────────────
    // OSRM espera: longitude,latitude (não lat,lng!)
    let url = format!(
        "{base}/{from_lng},{from_lat};{to_lng},{to_lat}\
         ?overview=false&alternatives=false&steps=false",
        base = osrm_base.trim_end_matches('/'),
    );

    // ── Chama OSRM ────────────────────────────────────────────────────────
    let upstream_req = Request::builder()
        .method(Method::Get)
        .uri(&url)
        .header("user-agent", "cardapio-geo-api/0.1")
        .header("accept", "application/json")
        .body("")
        .build();

    let upstream_resp: spin_sdk::http::Response = send(upstream_req)
        .await
        .context("falha ao contactar OSRM")
        .map_err(|e| ApiError::bad_gateway("osrm", e.to_string()))?;

    if *upstream_resp.status() != 200 {
        return Err(ApiError::bad_gateway(
            "osrm",
            format!("status HTTP {}", upstream_resp.status()),
        ));
    }

    // ── Parseia resposta OSRM ─────────────────────────────────────────────
    let raw = std::str::from_utf8(upstream_resp.body())
        .context("resposta do OSRM não é UTF-8")
        .map_err(ApiError::from)?;

    let osrm: serde_json::Value = serde_json::from_str(raw)
        .context("resposta do OSRM inválida")
        .map_err(ApiError::from)?;

    let code = osrm["code"].as_str().unwrap_or("");
    if code != "Ok" {
        return Err(ApiError::bad_gateway(
            "osrm",
            format!(
                "código de erro: {}",
                osrm["message"].as_str().unwrap_or(code)
            ),
        ));
    }

    let route = &osrm["routes"][0];
    let distance_m = route["distance"].as_f64().unwrap_or(0.0);
    let duration_s = route["duration"].as_f64().unwrap_or(0.0);

    // Arredonda para 1 decimal — evita ruído de precisão no frontend.
    let distance_km = (distance_m / 1000.0 * 10.0).round() / 10.0;
    let duration_min = (duration_s / 60.0 * 10.0).round() / 10.0;

    let body = serde_json::json!({
        "ok":             true,
        "out_of_range":   false,
        "distance_km":    distance_km,
        "duration_min":   duration_min,
        "within_alagoas": to_al,
        "within_region":  to_ext,
        // routes[0].distance em metros — compatibilidade com código que usa OSRM direto
        "routes": [{ "distance": distance_m, "duration": duration_s }]
    });

    let body_str = serde_json::to_string(&body).map_err(anyhow::Error::from)?;
    Ok(json_ok(body_str))
}