# Para el Director de Tecnología

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Arquitectura

- **Servicio:** Rust sobre Axum y axa (sqlx 0.9), PostgreSQL 18, Redis para caché,
  contadores compartidos y propagación del acceso de emergencia. `forbid(unsafe_code)`, clippy
  pedantic. La fuente de verdad del comportamiento es la especificación
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Interfaz:** SvelteKit 3 / Svelte 5, un proxy de backend para frontend para que ningún token llegue
  nunca al JavaScript del navegador, CSP estricta con hashes.
- **Autenticación:** PASETO v4.public verificado sin conexión, Keycloak opcional; control de acceso
  basado en atributos (acciones de leer/escribir/eliminar/destructivas, una obligación `mask`,
  verificaciones a nivel de registro) desde un motor compartido.
- **Auditoría:** cadena de hashes SHA-256 escrita en la misma transacción que el cambio,
  disparador de solo anexado, verificable en `GET /api/audits/verify`.
- **Protección de datos:** cifrado de campo AES-256-GCM con rotación de claves en dos
  columnas; claves locales, sin KMS.

## Evidencia de calidad

Pruebas unitarias (185), pruebas de base de datos (56, incluidas cumplimiento de reglas, enmascaramiento, auditoría,
paginación, caché, idempotencia, verificaciones de esquema), pruebas unitarias del front-end (106) y
Playwright (98), incluidos análisis de accesibilidad con axe. CI ejecuta fmt, clippy, pruebas, compilación y lint del front-end y
una etapa de cadena de suministro (`cargo audit`, `cargo deny`, `pnpm audit`, SBOM CycloneDX)
en GitHub Actions y Woodpecker. Un solo comando lo reproduce: `scripts/ci-check.sh`.

## Deuda técnica y límites honestos

- La paginación usa un cursor por desplazamiento (las filas pueden omitirse o repetirse si los datos cambian
  durante el recorrido); sin conteo total por diseño.
- Las claves de idempotencia son opcionales en la API (T52): un cliente debe enviar
  `Idempotency-Key` en una creación para estar protegido de que un reintento duplique un
  registro. La interfaz incluida envía una en cada creación.
- Los esquemas de OpenAPI están escritos a mano. Cubren solicitudes, listas, lecturas de detalle,
  tableros, vistas de analítica, auditoría, privacidad y acceso de emergencia; las transiciones de estado
  y los upserts que devuelven el registro actualizado no están tipados. Una prueba falla cuando una
  ruta y el documento no coinciden, y otras comprueban respuestas reales, incluidas las enmascaradas,
  contra los esquemas
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Los errores son documentos de problema RFC 9457 (T54) que conservan los miembros anteriores `error` y
  `description`; los errores de capas fuera del enrutador (límite de tamaño,
  tiempo de espera) no se convierten. La política de versionado y obsolescencia es
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); los encabezados
  `Deprecation` y `Sunset` están especificados pero no construidos.
- El cifrado de campo cubre dos columnas; sin integración con KMS.
- El estado del límite de tasa y de la caché necesita Redis para ser correcto con varias réplicas.
- Las vistas de analítica se enmascaran por rol, pero se guardan en caché por token de solicitante durante 30 s.
- La reautenticación escalonada antes de acciones destructivas requiere trabajo con el proveedor de identidad.

## Secuencia técnica recomendada

1. Claves de campo respaldadas por KMS y mayor cobertura de campos (resto de T41).
2. Etiquetar una primera versión para demostrar la firma sin llaves y la verificación (T49), y
   pedir a un revisor que compruebe el borrador de estándares de la API.
3. Construir los encabezados `Deprecation` y `Sunset` con la primera obsolescencia.
4. Encargar la prueba independiente (T42) sobre una compilación de preproducción.
