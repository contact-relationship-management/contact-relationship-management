# Gobernanza, Riesgo y Cumplimiento

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Por dónde empezar

El paquete de evidencia es [spec/compliance](../../../../spec/compliance/index.md). Su
regla básica es la que debe exigirse al equipo: un control está **Implementado** solo
cuando el código o la configuración lo aplican y una prueba o verificación lo respalda; **Quien implementa**
cuando su organización debe aportarlo; **Brecha** cuando hoy no se cumple ninguna de las dos.

| Documento | Para qué usarlo |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, Regla de Seguridad de HIPAA, Anexo A de ISO 27001, Art. 32 del RGPD, cada uno marcado como Implementado, Quien implementa o Brecha |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Evaluación de impacto en la protección de datos, rellenada previamente con los flujos de datos de este sistema |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Endurecimiento, copia de seguridad y restauración, claves, registros, respuesta a incidentes |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Borradores de declaraciones DTAC del NHS para que el fabricante los adopte |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Autoevaluación frente a los estándares de API del gobierno del Reino Unido (un borrador, no contrastado con el texto publicado) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Formato de errores (RFC 9457), qué cuenta como cambio incompatible, los pasos de obsolescencia y el plazo de aviso |
| [Evaluación NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Resultado pregunta por pregunta: probablemente fuera del alcance de DTAC, y no aprobaría las secciones aplicables por falta de evidencia organizacional |
| [plan](../../../../spec/plan.md) | Veredicto de preparación (no listo) y registro de brechas |
| [tasks](../../../../spec/tasks.md) | La tarea de cada brecha, su resolución y los límites declarados |

## Gobernanza

- **Derechos de decisión:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Las reglas de contribución y de asistencia con IA están en
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) y [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Control de cambios:** la especificación es la fuente de verdad; el código y las pruebas
  la citan. CI se ejecuta en dos proveedores y `scripts/ci-check.sh` lo reproduce.
  Regla del repositorio: no debilitar una prueba ni una compuerta para que algo pase.
- **Divulgación:** [SECURITY.md](../../../../SECURITY.md) es la vía para reportar.

## Riesgo

La vista de riesgos está en el [resumen del Director de Riesgos](../director-de-riesgos/index.md). En resumen: los mayores
riesgos residuales son la seguridad sin probar por terceros (T42), la
accesibilidad sin cuantificar (T48) y la clasificación de datos, que decide quien adopta.

## Cumplimiento: qué es real y qué no

| Área | Posición |
|---|---|
| Control de acceso | Implementado y probado: enmascaramiento por rol, verificaciones a nivel de registro, lecturas operativas solo para administradores, acceso de emergencia con revisión obligatoria por una segunda persona |
| Auditoría | Implementada y probada: encadenada por hashes, de solo anexado, dentro de la transacción, auditoría de lecturas incluidas listas, listas de negociaciones y analítica. **Quien implementa:** anclar la cabeza de la cadena fuera de la base de datos |
| Derechos de privacidad | Implementados: exportación de acceso del titular, supresión (rechazada mientras haya obligaciones abiertas), barrido de retención. **Quien implementa:** base legal, calendario de retención, registros de tratamiento |
| Cifrado | Parcial: AES-256-GCM a nivel de campo en dos columnas con claves locales, sin KMS. El cifrado en reposo y en tránsito corresponde a quien implementa |
| Cadena de suministro | Implementada: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, imágenes base fijadas por resumen criptográfico, actualizaciones de Dependabot. **Brecha:** el flujo de versiones firmadas no se ha ejecutado en una versión real (T49) |
| Accesibilidad | Parcial: análisis automáticos de axe y una declaración pública honesta. **Brecha:** sin auditoría manual (T48), traducciones sin revisar |
| Aseguramiento independiente | **Brecha:** ninguno (T42) |
| Ley de correo y marketing | Ruta de envío que parte del consentimiento implementada; el envío real existe solo como una ruta SMTP para desarrollo; existen el enlace de baja, la lista de supresión y un webhook firmado de resultados, pero no hay proveedor integrado y el acuerdo de encargado y la revisión legal siguen abiertos (T39) |

## Controles que necesitan un responsable en su organización

1. Revisión y alertas de las concesiones de acceso de emergencia (alertar sobre `break_glass_granted`).
2. Anclaje de la cabeza de la cadena de auditoría y su verificación periódica
   (`GET /api/audits/verify`).
3. Evidencia de copia de seguridad y restauración, y ensayo de rotación de claves.
4. Gestión de altas, cambios y bajas en su proveedor de identidad.
5. Retención de la tabla de auditoría, que crece con el tráfico de lecturas.
6. Protección de Redis como la base de datos: guarda datos de analítica en caché.
7. Fusión de las actualizaciones de Dependabot y verificación de una versión firmada antes de desplegarla.

## Qué negarse a avalar

Cualquier afirmación de que el sistema cumple, está certificado, cumple con HIPAA o está aprobado
por el NHS. La declaración defendible es más acotada: los controles están mapeados, los
implementados están probados y las brechas están identificadas y en seguimiento.
