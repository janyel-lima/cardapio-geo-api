/// GET /route?from_lat=<f>&from_lng=<f>&to_lat=<f>&to_lng=<f>
///
/// Cálculo de rota via OSRM.
///
/// A URL base do OSRM é lida da variável Spin `osrm_base`, definida em spin.toml.
/// Pode ser sobrescrita em runtime via:
///
///   # dev container — OSRM local
///   export SPIN_VARIABLE_OSRM_BASE="http://localhost:5000/route/v1/driving"
///
///   # docker compose — o Spin container alcança o serviço "osrm" pelo nome
///   environment:
///     SPIN_VARIABLE_OSRM_BASE: http://osrm:5000/route/v1/driving
///
///   # padrão (sem configuração) — API pública do Project OSRM
///   https://router.project-osrm.org/route/v1/driving
///
/// Resposta compatível com address.js:
///
/// ```json
/// {
///   "ok":             true,
///   "distance_km":    5.3,
///   "duration_min":   12.0,
///   "within_alagoas": true,
///   "within_region":  true,
///   "routes": [{ "distance": 5300.0, "duration": 720.0 }]
/// }
/// ```
///
/// `routes[0].distance` em metros — idêntico ao OSRM bruto.

use anyhow::Context;
use spin_sdk::http::{Method, Request, Response, send};
use spin_sdk::variables;

use crate::bounds;
use crate::helpers::{error_json, json_ok, parse_qs, require_f64};

pub async fn handle(query: &str) -> anyhow::Result<Response> {
    let qs = parse_qs(query);

    // ── Lê a URL base do OSRM da variável Spin ────────────────────────────
    // Definida em spin.toml → [variables] osrm_base
    // Sobrescrita via SPIN_VARIABLE_OSRM_BASE no ambiente de execução
    let osrm_base = variables::get("osrm_base")
        .context("variável 'osrm_base' não definida no spin.toml")?;

    // ── Valida parâmetros ─────────────────────────────────────────────────
    let from_lat = match require_f64(&qs, "from_lat") {
        Ok(v) => v,
        Err(e) => return Ok(error_json(400, &e.to_string())),
    };
    let from_lng = match require_f64(&qs, "from_lng") {
        Ok(v) => v,
        Err(e) => return Ok(error_json(400, &e.to_string())),
    };
    let to_lat = match require_f64(&qs, "to_lat") {
        Ok(v) => v,
        Err(e) => return Ok(error_json(400, &e.to_string())),
    };
    let to_lng = match require_f64(&qs, "to_lng") {
        Ok(v) => v,
        Err(e) => return Ok(error_json(400, &e.to_string())),
    };

    // ── Valida região ─────────────────────────────────────────────────────
    let (_, from_ext) = bounds::classify(from_lat, from_lng);
    let (to_al, to_ext) = bounds::classify(to_lat, to_lng);

    if !from_ext {
        return Ok(error_json(
            422,
            "origem está fora da região operacional (Alagoas e limítrofes)",
        ));
    }
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
        return Ok(json_ok(serde_json::to_string(&body)?));
    }

    // ── Monta URL OSRM ────────────────────────────────────────────────────
    // OSRM espera: longitude,latitude (nessa ordem!)
    let url = format!(
        "{base}/{from_lng},{from_lat};{to_lng},{to_lat}\
         ?overview=false&alternatives=false&steps=false",
        base     = osrm_base.trim_end_matches('/'),
        from_lng = from_lng,
        from_lat = from_lat,
        to_lng   = to_lng,
        to_lat   = to_lat,
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
        .context("falha ao contactar OSRM")?;

    if *upstream_resp.status() != 200 {
        return Ok(error_json(
            502,
            &format!("OSRM retornou status {}", upstream_resp.status()),
        ));
    }

    // ── Parseia resposta OSRM ─────────────────────────────────────────────
    let raw = std::str::from_utf8(upstream_resp.body())
        .context("resposta do OSRM não é UTF-8")?;

    let osrm: serde_json::Value =
        serde_json::from_str(raw).context("resposta do OSRM inválida")?;

    let code = osrm["code"].as_str().unwrap_or("");
    if code != "Ok" {
        return Ok(error_json(
            502,
            &format!(
                "OSRM reportou erro: {}",
                osrm["message"].as_str().unwrap_or(code)
            ),
        ));
    }

    let route        = &osrm["routes"][0];
    let distance_m   = route["distance"].as_f64().unwrap_or(0.0);
    let duration_s   = route["duration"].as_f64().unwrap_or(0.0);
    let distance_km  = (distance_m  / 1000.0 * 10.0).round() / 10.0;
    let duration_min = (duration_s  / 60.0   * 10.0).round() / 10.0;

    let body = serde_json::json!({
        "ok":             true,
        "out_of_range":   false,
        "distance_km":    distance_km,
        "duration_min":   duration_min,
        "within_alagoas": to_al,
        "within_region":  to_ext,
        "routes": [{
            "distance": distance_m,
            "duration": duration_s,
        }]
    });

    Ok(json_ok(serde_json::to_string(&body)?))
}