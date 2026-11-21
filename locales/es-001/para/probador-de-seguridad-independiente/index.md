# Probador de Seguridad Independiente

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Por qué existe esto

Cada control de seguridad de este proyecto fue construido y probado por sus autores
(CRM-T42). Eso demuestra que los controles funcionan como los autores los entendieron. No
demuestra que no falte nada. Los hospitales y los compradores gubernamentales suelen exigir
una prueba de alguien independiente antes de guardar datos reales, y la afirmación honesta
hasta entonces es «construido con un alto estándar, sin probar por terceros»
([resumen del Director Ejecutivo](../director-ejecutivo/index.md),
[resumen de seguridad](../director-de-seguridad/index.md)).

Esta página es lo que se entrega a una firma de pruebas y lo que se debe esperar de vuelta.

## Qué significa aprobar y qué no

Un informe limpio dice que un equipo identificado, en un tiempo determinado y con un alcance
determinado, no encontró problemas graves. **No** es una certificación, ni una acreditación ni una
garantía. No describa el sistema como «seguro», «certificado» ni «probado con pruebas de penetración
conforme a un estándar» más allá de lo que diga el informe
([resumen de GRC](../gobernanza-riesgo-cumplimiento/index.md)).

## Cómo elegir al probador

- Independiente de los autores y de cualquiera que haya configurado la instalación.
- Con personal identificado y experimentado, con la acreditación pertinente para su
  mercado cuando aplique (por ejemplo CREST, o el esquema CHECK del NCSC para
  trabajo del gobierno del Reino Unido). Pregunte qué acreditación cubre a las personas que hacen
  el trabajo, no solo a la empresa.
- Acepta por escrito el alcance, las reglas de contratación, la confidencialidad, el manejo
  de datos y la responsabilidad antes de empezar cualquier prueba. El contrato lo
  acuerda el equipo jurídico ([resumen del Director Jurídico](../director-jurídico/index.md)).

## Qué probar

| Área | Qué deben intentar los probadores | Dónde se describe el diseño |
|---|---|---|
| Autenticación | Tokens falsificados, vencidos, repetidos y de audiencia equivocada; degradación a no autenticado; la verificación de inicio que se niega a ejecutar con la autenticación desactivada | [auth](../../../../spec/auth.md) |
| Autorización | Alcanzar registros o acciones que un rol no debería: escalada horizontal y vertical, verificaciones a nivel de registro, acciones destructivas, lecturas solo para administradores (auditoría, retención, revisión de acceso de emergencia) | [auth](../../../../spec/auth.md) |
| Exposición de datos | Valores enmascarados que se filtran por listas, lecturas de detalle agrupadas, analítica, agregados, mensajes de error, caché o la clave de caché | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Acceso de emergencia | Tomar, extender o abusar del acceso de emergencia; revisar su propia concesión; revisar dos veces | [auth](../../../../spec/auth.md) (T44) |
| Auditoría | Alterar, eliminar o reordenar el historial; eludir la cadena; ocultar lecturas | [audit](../../../../spec/audit.md) |
| Resistencia al abuso | Elusión del límite de tasa y del bloqueo, bloquear a otros usuarios, suplantación de encabezados para la clave del cliente | T43 |
| Manejo de entradas | Inyección (SQL, JSON, encabezados), cuerpos sobredimensionados, cursores y claves de idempotencia mal formados, repetir o colisionar claves entre solicitantes | T51, T52 |
| Front-end web | Cross-site scripting, falsificación de solicitudes, manejo de sesiones, el proxy de backend para frontend, política de seguridad de contenido, clickjacking | `hooks.server.ts` de la interfaz |
| Infraestructura tal como se distribuye | Imágenes de contenedor, puertos expuestos, accesibilidad de Redis y PostgreSQL, manejo de secretos, valores de configuración predeterminados | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Cadena de suministro | Procedencia de dependencias e imágenes, las SBOM, compilaciones sin firmar | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Lógica de negocio | Elusión del consentimiento en la ruta de envío, supresión que deja datos, manipulación de la puntuación, creación duplicada | [regulatory](../../../../spec/regulatory.md) |

**Fuera de alcance salvo que se acuerde otra cosa:** el servicio de identidad que emite
los tokens (está fuera de este repositorio), la plataforma de alojamiento y la denegación de servicio
a gran volumen.

## Qué entregar a los probadores

1. Una **compilación de preproducción** que coincida con lo que se ejecutará en producción: configuración
   de producción, autenticación activada, TLS, un Redis y un PostgreSQL reales. No pruebe
   los valores predeterminados de la demostración.
2. **Solo datos sintéticos.** La tarea de semilla construye una cartera de negocios sin personas
   reales. Nunca deben existir datos reales de pacientes o ciudadanos en un entorno de pruebas.
3. **Una cuenta por rol**, para probar la autorización desde cada lado. La
   política de referencia es `config/abac-policy.reference.json`.
4. Los documentos enlazados arriba, el documento OpenAPI en `/api-docs/openapi.json`
   y acceso de lectura al repositorio, para que la prueba combine caja negra y revisión
   de código.
5. Las **debilidades conocidas**, para que los probadores dediquen su tiempo a lo que aún no está
   documentado:
   - la firma de versiones está construida pero no se ha ejecutado en una versión real (T49);
   - el cifrado de campo cubre dos columnas con claves locales, sin KMS;
   - no está construida la reautenticación escalonada;
   - el acceso de emergencia no puede revocarse, solo vencer, y no hay alerta conectada;
   - las recargas de políticas se reflejan en la analítica en caché hasta 30 segundos después;
   - el cierre de sesión por inactividad ocurre en el cliente;
   - el estado del límite de tasa recurre a conteo por réplica si Redis falla.
   Pídales que confirmen o refuten cada una, no que las omitan.

## Reglas de contratación que se deben acordar por escrito

- Probadores, fechas y direcciones de origen identificados; un contacto disponible las 24 horas de su lado.
- Qué se puede intentar (por ejemplo, no destruir el almacén de datos de prueba
  sin aviso) y qué debe detenerse al demostrar la exposición.
- A quién se informa de inmediato de un hallazgo crítico, y cómo, antes del informe final.
- Cómo se almacenan y destruyen el informe, la evidencia y cualquier nota de los probadores.

## Qué debe recibir de vuelta

- Un informe con alcance, método, fechas, nombres de los probadores y acreditación.
- Cada hallazgo con severidad, evidencia, pasos para reproducirlo, componente afectado
  y una corrección recomendada.
- Una declaración clara de lo que **no** se probó.
- Una nueva prueba de los hallazgos corregidos, incluida en el contrato o con precio.

## Después del informe

1. Clasifique cada hallazgo en `spec/tasks.md` como una tarea con responsable y fecha.
2. Corrija los hallazgos críticos y altos antes de cualquier dato real y luego pida a los probadores
   que confirmen las correcciones.
3. Agregue una prueba de regresión por cada corrección. La regla del proyecto es no debilitar una prueba
   ni una compuerta para que algo pase.
4. Registre el resultado con honestidad: la fecha del informe, el alcance, los hallazgos y
   su estado. Actualice el plan de preparación ([plan](../../../../spec/plan.md)) y
   el paquete de cumplimiento ([cumplimiento](../../../../spec/compliance/index.md)).
5. Vuelva a probar tras cambios importantes y con la periodicidad que defina su responsable de riesgos.
   Una prueba es una instantánea, no un estado.

## Orden del trabajo

1. Cierre primero lo barato y conocido, para que los probadores no gasten el presupuesto
   redescubriéndolo: una primera versión firmada (T49), alertas sobre el acceso de emergencia
   y sobre fallos de verificación de la auditoría, Redis detrás de TLS y credenciales.
2. Prepare la compilación de preproducción, las cuentas y los datos semilla.
3. Acuerde el alcance, el contrato y las reglas de contratación.
4. Pruebe; reciba de inmediato los hallazgos críticos.
5. Corrija, vuelva a probar y registre.
6. Solo entonces actualice las afirmaciones externas, y solo con lo que respalde el informe.

## Decisiones para usted

1. Quién encarga y quién es dueño de los hallazgos (consulte el
   [resumen del Director de Riesgos](../director-de-riesgos/index.md)).
2. Qué acreditación espera su mercado de los probadores.
3. El presupuesto y la fecha, definidos por lo que cuesta un retraso y no por lo que
   resulte conveniente.
4. Si probar una sola vez antes del lanzamiento o con una periodicidad recurrente.
