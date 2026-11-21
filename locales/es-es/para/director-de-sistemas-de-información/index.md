# Para el Director de Sistemas de Información

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Vista de información y datos

Este CRM guarda **datos de relaciones**: contactos y cuentas como envoltorios de
identificadores que residen en otros sistemas, actividades, negociaciones, incidencias, historial de consentimiento
y una bitácora de auditoría. Los datos demográficos permanecen deliberadamente en los registros de identidad
([domain-model](../../../../spec/domain-model.md)). Si su organización registra pacientes o
ciudadanos como contactos, la clasificación de esos datos **le corresponde** a usted;
el plan está escrito con la lectura más estricta ([plan](../../../../spec/plan.md)).

## Adónde van los datos

| Almacén | Contiene | Protección en el repositorio |
|---|---|---|
| PostgreSQL | Todos los registros, la cadena de auditoría, las concesiones de acceso de emergencia | Cifrado de campo en dos columnas; disparador de auditoría de solo anexado; el cifrado en reposo corresponde a quien implementa |
| Redis | Caché de respuestas de analítica (30 s), contadores de límite de tasa, claves de concesión de acceso de emergencia | Con clave por solicitante; **requiere TLS, contraseña y ninguna exposición pública** |
| Registros/métricas | Metadatos de solicitudes, métricas de Prometheus | La retención corresponde a quien implementa |
| Proveedor de correo (futuro) | Contenido de los mensajes y destinatarios | No construido (T39) |

## Integración e identidad

- El inicio de sesión proviene de un servicio de identidad que emite tokens PASETO, o de Keycloak.
  Por lo tanto, altas/cambios/bajas de personal, MFA y política de sesión **corresponden a su proveedor
  de identidad**; el CRM aplica lo que dice el token y rechaza los vencidos.
- La API está documentada en OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)),
  versionada por encabezado, paginada con encabezados `Link` y devuelve los errores como
  documentos de problema RFC 9457. Las reglas de cambio y obsolescencia están en
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- Las SBOM (CycloneDX) se generan para el servicio y la interfaz en CI; el flujo de versiones
  las firma junto con las imágenes.

## Obligaciones de gobierno de la información que siguen a su cargo

Evidencia de copia de seguridad y restauración, calendarios de retención por tabla, anclaje de la cabeza de la cadena
de auditoría fuera de la base de datos, custodia en KMS de las claves de campo, acuerdos de tratamiento de datos
con cualquier proveedor y la evaluación de impacto en la protección de datos
([dpia-template](../../../../spec/compliance/dpia-template.md)). El mapeo de controles
([control-mapping](../../../../spec/compliance/control-mapping.md)) marca cada control como
Implementado, Quien implementa o Brecha.
