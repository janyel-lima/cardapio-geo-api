use std::collections::HashMap;

use spin_sdk::http::Response;

// ── Query string ──────────────────────────────────────────────────────────

/// Decodifica um componente percent-encoded da URL.
///
/// `+` → espaço, `%XX` → byte correspondente.
///
/// # Correção UTF-8
/// A implementação coleta **bytes** antes de converter para `String`.
/// A versão anterior convertia cada byte diretamente para `char`, o que
/// produzia lixo para sequências multibyte (ex: `%C3%A7` → ç).
pub fn url_decode(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let raw = s.as_bytes();
    let mut i = 0;

    while i < raw.len() {
        match raw[i] {
            b'+' => {
                bytes.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < raw.len() => {
                // Nota: &s[i+1..i+3] é seguro porque os bytes já são ASCII hex.
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    bytes.push(byte);
                    i += 3;
                } else {
                    bytes.push(b'%');
                    i += 1;
                }
            }
            c => {
                bytes.push(c);
                i += 1;
            }
        }
    }

    // from_utf8_lossy substitui sequências inválidas por U+FFFD em vez de panic.
    String::from_utf8_lossy(&bytes).into_owned()
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

/// Parseia `f64` de um parâmetro, retornando erro amigável se ausente/inválido.
pub fn require_f64(qs: &HashMap<String, String>, key: &str) -> anyhow::Result<f64> {
    let raw = qs
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("parâmetro '{}' ausente", key))?;
    raw.parse::<f64>()
        .map_err(|_| anyhow::anyhow!("parâmetro '{}' inválido: '{}'", key, raw))
}

/// Parseia `String` obrigatória de um parâmetro.
pub fn require_str<'a>(qs: &'a HashMap<String, String>, key: &str) -> anyhow::Result<&'a str> {
    qs.get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("parâmetro '{}' ausente", key))
}

// ── Response builders ─────────────────────────────────────────────────────

const CORS_ORIGIN: &str = "*";
const CORS_METHODS: &str = "GET, OPTIONS";
const CORS_HEADERS: &str = "Content-Type";

/// Resposta JSON 200 com headers CORS e cache de 5 minutos.
pub fn json_ok(body: String) -> Response {
    Response::builder()
        .status(200)
        .header("content-type", "application/json; charset=utf-8")
        .header("access-control-allow-origin", CORS_ORIGIN)
        .header("access-control-allow-methods", CORS_METHODS)
        .header("access-control-allow-headers", CORS_HEADERS)
        .header("cache-control", "public, max-age=300")
        .body(body)
        .build()
}

/// Resposta de erro JSON com status arbitrário.
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
        .header("access-control-allow-origin", CORS_ORIGIN)
        .header("access-control-allow-methods", CORS_METHODS)
        .header("access-control-allow-headers", CORS_HEADERS)
        .header("access-control-max-age", "86400")
        .body("")
        .build()
}

/// Health check.
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
        &format!(
            "rota '{}' não encontrada. Endpoints: /geocode /reverse /route /health",
            path
        ),
    )
}

// ── Testes ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── url_decode ──────────────────────────────────────────────────────

    #[test]
    fn url_decode_plain_ascii() {
        assert_eq!(url_decode("Arapiraca"), "Arapiraca");
    }

    #[test]
    fn url_decode_plus_as_space() {
        assert_eq!(url_decode("Rua+das+Flores"), "Rua das Flores");
    }

    #[test]
    fn url_decode_percent_ascii() {
        assert_eq!(url_decode("al%3Dmaceio"), "al=maceio");
    }

    #[test]
    fn url_decode_utf8_multibyte() {
        // %C3%A7 = ç (U+00E7, dois bytes UTF-8)
        // Este teste falhava na implementação anterior que usava `byte as char`.
        assert_eq!(url_decode("Ara%C3%A7atuba"), "Araçatuba");
    }

    #[test]
    fn url_decode_mixed() {
        assert_eq!(
            url_decode("Rua+S%C3%A3o+Jo%C3%A3o"),
            "Rua São João"
        );
    }

    #[test]
    fn url_decode_invalid_percent_passthrough() {
        // % sem dois hex válidos → mantém o %
        assert_eq!(url_decode("test%ZZend"), "test%ZZend");
    }

    // ── parse_qs ────────────────────────────────────────────────────────

    #[test]
    fn parse_qs_basic() {
        let qs = parse_qs("q=Arapiraca&limit=3");
        assert_eq!(qs["q"], "Arapiraca");
        assert_eq!(qs["limit"], "3");
    }

    #[test]
    fn parse_qs_empty_string() {
        assert!(parse_qs("").is_empty());
    }

    #[test]
    fn parse_qs_key_without_value() {
        let qs = parse_qs("foo=&bar=baz");
        assert_eq!(qs["foo"], "");
        assert_eq!(qs["bar"], "baz");
    }

    #[test]
    fn parse_qs_decodes_values() {
        let qs = parse_qs("q=S%C3%A3o+Paulo");
        assert_eq!(qs["q"], "São Paulo");
    }

    // ── require_f64 ─────────────────────────────────────────────────────

    #[test]
    fn require_f64_valid_negative() {
        let mut m = HashMap::new();
        m.insert("lat".to_string(), "-9.7514".to_string());
        let v = require_f64(&m, "lat").unwrap();
        assert!((v - (-9.7514)).abs() < f64::EPSILON);
    }

    #[test]
    fn require_f64_missing_key_is_err() {
        assert!(require_f64(&HashMap::new(), "lat").is_err());
    }

    #[test]
    fn require_f64_non_numeric_is_err() {
        let mut m = HashMap::new();
        m.insert("lat".to_string(), "abc".to_string());
        assert!(require_f64(&m, "lat").is_err());
    }
}