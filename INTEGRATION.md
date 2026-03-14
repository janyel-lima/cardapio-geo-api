# Integração com o Cardápio Digital Pro

Duas mudanças no `address.js` são suficientes para apontar para a nova API.
O restante do código permanece idêntico.

---

## 1. Constante de base URL (adicione no topo do arquivo)

```js
// ── URL base da API de geocoding/rotas ────────────────────────────────────
// Dev local:   'http://localhost:3000'
// Produção:    'https://cardapio-geo-api.SEU-USUARIO.fermyon.app'
const GEO_API = 'http://localhost:3000';
```

---

## 2. `fetchDeliveryRoute()` — troca OSRM pela nova API

**Antes (OSRM direto):**
```js
async fetchDeliveryRoute() {
  const { storeLat, storeLng } = /* ... */;
  const { lat, lng } = this.deliveryCoords;

  const url = `https://router.project-osrm.org/route/v1/driving/`
    + `${storeLng},${storeLat};${lng},${lat}?overview=false`;

  const resp = await fetch(url);
  const data = await resp.json();

  const distanceKm = data.routes[0].distance / 1000;
  // ...
}
```

**Depois (nova API):**
```js
async fetchDeliveryRoute() {
  const { storeLat, storeLng } = this.config;
  const { lat, lng } = this.deliveryCoords;

  const url = `${GEO_API}/route`
    + `?from_lat=${storeLat}&from_lng=${storeLng}`
    + `&to_lat=${lat}&to_lng=${lng}`;

  const resp = await fetch(url);
  const data = await resp.json();

  if (!data.ok && data.out_of_range) {
    // A API já indicou que está fora de área — sem precisar checar zonas aqui
    this.deliveryRouteKm = null;
    this._setDeliveryOutOfRange(true);
    return;
  }

  // Compatibilidade: data.routes[0].distance ainda existe em metros
  const distanceKm = data.distance_km;          // campo novo, mais conveniente
  // ou: data.routes[0].distance / 1000         // igual ao que era antes

  this.deliveryRouteKm = distanceKm;
  // ... resto do handler igual
}
```

> **Nota:** a resposta ainda inclui `routes[0].distance` (metros) para total
> compatibilidade. Se o seu `address.js` atual lê `data.routes[0].distance / 1000`,
> ele continua funcionando sem nenhuma alteração nessa linha.

---

## 3. Leaflet Geocoder — troca Nominatim pela nova API

Se você usa o plugin `Leaflet.Control.Geocoder`, basta trocar o provider:

**Antes:**
```js
L.Control.geocoder({
  geocoder: L.Control.Geocoder.nominatim()
}).addTo(map);
```

**Depois:**
```js
L.Control.geocoder({
  geocoder: L.Control.Geocoder.nominatim({
    serviceUrl: `${GEO_API}/geocode`,   // /geocode é compatível com Nominatim jsonv2
  })
}).addTo(map);
```

> O endpoint `/geocode` retorna exatamente o formato `jsonv2` do Nominatim.
> O plugin não percebe diferença.

---

## 4. `_reverseGeocode()` — troca Nominatim reverse pela nova API

**Antes:**
```js
const url = `https://nominatim.openstreetmap.org/reverse`
  + `?lat=${lat}&lon=${lng}&format=jsonv2&addressdetails=1`;
```

**Depois:**
```js
const url = `${GEO_API}/reverse?lat=${lat}&lng=${lng}`;
```

> O schema da resposta é idêntico ao Nominatim `/reverse`.
> O código que lê `data.address.road`, `data.address.city`, etc. não muda.

---

## Resumo das diferenças de resposta

| Campo | OSRM/Nominatim direto | Nova API |
|-------|-----------------------|----------|
| `routes[0].distance` | ✅ metros | ✅ metros (passthrough) |
| `distance_km` | ❌ | ✅ km arredondado (1 decimal) |
| `duration_min` | ❌ | ✅ minutos |
| `out_of_range` | ❌ (só inferido via zonas) | ✅ direto na resposta |
| `within_alagoas` | ❌ | ✅ |
| `address.*` | ✅ | ✅ (idêntico) |
| Headers CORS | ❌ (Nominatim não envia) | ✅ (`*`) |
| Cache-Control | ❌ | ✅ `public, max-age=300` |