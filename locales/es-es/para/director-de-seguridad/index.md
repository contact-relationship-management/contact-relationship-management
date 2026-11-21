# Para el Director de Seguridad

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Postura de seguridad en un párrafo

El servicio **falla de forma segura**, controla el acceso por rol y por registro, oculta
los valores sensibles a los roles que no deben verlos y mantiene una pista de auditoría
a prueba de manipulación. Esos controles están implementados y probados por los autores. **No han sido probados
por nadie independiente** (T42), así que trate cada afirmación siguiente
como «construido y autoprobado», no como «verificado».

## Controles y dónde comprobarlos

| Área | Qué existe | Dónde mirar |
|---|---|---|
| Autenticación | PASETO v4.public verificado sin conexión, Keycloak opcional; producción se niega a iniciar con la autenticación desactivada | [auth](../../../../spec/auth.md) |
| Autorización | Basada en atributos: leer, escribir, eliminar, destructivas; una obligación `mask`; verificaciones a nivel de registro; auditoría, retención y revisión de acceso de emergencia solo para administradores | [auth](../../../../spec/auth.md), prueba de matriz de personas |
| Exposición de datos | Enmascaramiento en registros, agregados y analítica; se encontró y corrigió una fuga en el detalle de cuentas (T47) | [tasks](../../../../spec/tasks.md) |
| Acceso de emergencia | Motivo escrito, tope de 60 minutos, solo lectura, revisión obligatoria por una segunda persona, auditado, compartido mediante Redis | tareas de break-glass T44 |
| Auditoría | Cadena de hashes SHA-256 en la misma transacción, disparador de solo anexado, endpoint de verificación; lecturas auditadas | [audit](../../../../spec/audit.md) |
| Resistencia al abuso | Límite de tasa (600 por minuto) y bloqueo (10 fallos en 5 minutos), compartidos mediante Redis; sondeos exentos | T43 |
| Navegador y transporte | CSP estricta con hashes, denegación de marcos, `nosniff`, política de referrer, CORS cerrado, encabezados seguros; ningún token en el JavaScript del navegador (BFF) | `hooks.server.ts` de la interfaz |
| Protección de datos | AES-256-GCM en dos columnas con rotación; claves locales, sin KMS | T41 |
| Cadena de suministro | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, imágenes endurecidas sin root fijadas por resumen criptográfico, actualizaciones semanales de Dependabot, un flujo de versiones que firma imágenes y SBOM sin llaves | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Errores | Cada error es un documento de problema RFC 9457; un `401` nombra el esquema aceptado | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Seguridad de memoria | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Debilidades conocidas (declaradas, no ocultas)

- **Sin prueba de penetración independiente ni ejercicio de modelado de amenazas** (T42). El
  modelo de amenazas de [dtac-supporting](../../../../spec/compliance/dtac-supporting.md)
  es un resumen, no uno estructurado.
- **La firma de versiones no se ha ejecutado en una versión real** (T49). El flujo y los
  comandos de verificación existen; hasta que se envíe una etiqueta están sin probar. Las acciones
  están fijadas por hash de commit.
- El **cifrado de campo** cubre dos columnas con claves locales, sin KMS.
- **No está construida** la reautenticación escalonada antes de acciones destructivas.
- **Redis guarda datos personales** (analítica en caché, claves de concesión) y debe protegerse
  como la base de datos: TLS, contraseña, sin exposición pública.
- El **acceso de emergencia** no puede revocarse, solo vencer, y no hay alerta conectada.
- El **límite de tasa y el bloqueo** recurren a conteos por réplica si Redis falla; el
  grupo no autenticado es por dirección, y detrás de un proxy no incluido en
  `TRUSTED_PROXIES` esa dirección es la del proxy, por lo que un atacante puede bloquear a otros
  solicitantes no autenticados. Un solicitante con un token válido se cuenta por su sujeto.
- Las **recargas de políticas** se reflejan en la analítica en caché hasta 30 segundos después.
- El **cierre de sesión por inactividad** ocurre en el cliente.
- Las **claves de idempotencia** son opcionales en la API (T52): un cliente que no envía
  `Idempotency-Key` no está protegido de que un reintento de creación duplique un registro.
  La interfaz incluida sí la envía.

## Qué debe hacer quien implementa

1. Mantener la autenticación activada; aportar claves de producción y `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Alertar sobre `break_glass_granted` y revisar las concesiones con prontitud.
3. Anclar la cabeza de la cadena de auditoría fuera de la base de datos y verificarla
   (`GET /api/audits/verify`) con una periodicidad definida.
4. Proteger PostgreSQL y Redis (red, TLS, credenciales, copias de seguridad, cifrado
   en reposo); rotar las claves de campo con la tarea incluida.
5. Fusionar las actualizaciones de Dependabot; volver a ejecutar la etapa de cadena de suministro; verificar una versión
   firmada antes de desplegarla.
6. Seguir el manual para la respuesta a incidentes
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Gestión de vulnerabilidades

Los reportes se envían mediante [SECURITY.md](../../../../SECURITY.md). Encargue la
prueba independiente (T42) sobre una compilación de preproducción antes de cualquier dato real, y
repita la prueba después de cambios importantes.

## Acciones de seguridad sugeridas, en orden

1. Definir el alcance y contratar la prueba de penetración independiente (T42).
2. Publicar una primera etiqueta de versión para demostrar la firma y la verificación (T49).
3. Conectar alertas para el acceso de emergencia y para fallos de verificación de la cadena de auditoría.
4. Poner Redis detrás de TLS y credenciales en todos los entornos.
5. Claves de campo respaldadas por KMS y mayor cobertura de campos (resto de T41).

Consulte también el resumen del [director de tecnología](../director-de-tecnología/index.md) y el del
[director de riesgos](../director-de-riesgos/index.md).
