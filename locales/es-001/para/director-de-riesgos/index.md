# Para el Director de Riesgos

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Posición de riesgo

**En general: no está listo para datos regulados.** El software está bien controlado para
su etapa, pero los principales riesgos residuales no se mitigan con pruebas ni procesos
que solo puede aportar la organización que lo adopte o terceros externos.

## Principales riesgos

| # | Riesgo | Base de la calificación | Mitigación en el repositorio | Residual |
|---|---|---|---|---|
| 1 | Vulnerabilidades sin descubrir | Sin prueba de penetración independiente (T42) | Pruebas propias, compuertas de cadena de suministro, encabezados/CSP estrictos | **Alto** hasta que se pruebe |
| 2 | Divulgación de campos sensibles al rol equivocado | Reglas de acceso complejas | Enmascaramiento por rol en registros, agregados y analítica; una prueba de matriz de personas; una fuga encontrada y corregida en el detalle de cuentas (T47) | Medio: el enmascaramiento está probado, no verificado de forma independiente |
| 3 | Mal uso interno del acceso de emergencia | El acceso de emergencia eleva las lecturas | Motivo escrito, tope de 60 minutos, solo lectura, revisión obligatoria por una segunda persona, auditado | Medio: no hay alerta conectada; no se puede revocar |
| 4 | Alteración de la pista de auditoría | Requiere acceso a la base de datos | Cadena de hashes más disparador de solo anexado; informa la primera ruptura | Bajo-Medio: el hash de la cabeza debe anclarse fuera de la base de datos |
| 5 | Marketing ilícito | Errores de consentimiento | La ruta de envío aplica el consentimiento, no la interfaz; historial de consentimiento inmutable | Medio hasta que se integre y revise un proveedor real (T39) |
| 6 | Disponibilidad / abuso | API pública | Límite de tasa y bloqueo, compartidos mediante Redis | Medio: los límites se aplican por réplica si Redis falla |
| 7 | Incumplimiento de accesibilidad | Obligación del sector público | Análisis automáticos de axe; declaración de accesibilidad honesta | **Sin cuantificar** hasta la auditoría manual (T48) |
| 8 | Cadena de suministro | Dependencias e imágenes | `cargo deny/audit`, `pnpm audit`, SBOM, resúmenes fijados y actualizados por Dependabot, un flujo de versiones con firma sin llaves | Medio: la firma no se ha ejecutado en una versión real (T49) |
| 9 | Registros duplicados por reintentos | Las claves de idempotencia son opcionales en la API (T52) | Repetición para clientes que envían la clave; la interfaz incluida siempre la envía | Bajo |
| 10 | Clasificación errónea de datos | Quien implementa decide qué es un «contacto» | Nota de alcance y plantilla de DPIA | Depende de usted |

## Brechas que quedan abiertas por naturaleza

T39 (proveedor de correo real), T42 (aseguramiento independiente), T48 (auditoría manual
de accesibilidad), T49 (una primera versión firmada, la revisión del borrador de la API por un revisor). Consulte [tasks.md](../../../../spec/tasks.md)
y [plan.md](../../../../spec/plan.md).

## Acciones de riesgo sugeridas

Registre formalmente los riesgos 1, 7 y 10; exija T42 antes de cualquier dato real; designe
un responsable de la revisión y las alertas del acceso de emergencia; fije la periodicidad del anclaje de la cabeza de auditoría.
