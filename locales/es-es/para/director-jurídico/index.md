# Para el Director Jurídico

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Postura legal en breve

El repositorio **no está listo para producción** y contiene datos sintéticos. No afirma
cumplir ningún régimen. Implementa funciones técnicas que apoyan
varios y enumera lo que quien lo implementa debe añadir ([regulatory](../../../../spec/regulatory.md)).

## Regímenes involucrados

| Régimen | Qué respalda el software | Qué debe aportar usted |
|---|---|---|
| RGPD del Reino Unido y la UE | Historial de consentimiento, exportación de acceso del titular, supresión (rechazada mientras haya obligaciones abiertas), barrido de retención, minimización de datos, auditoría de lecturas | Base legal por finalidad, DPIA ([plantilla](../../../../spec/compliance/dpia-template.md)), registros de tratamiento, calendario de retención, roles de responsable y encargado |
| PECR / e-privacy | Envíos condicionados al consentimiento, baja inmediata | Identificación del remitente y decisiones de «soft opt-in» para un remitente real |
| HIPAA (si hay datos de salud de EE. UU.) | Funciones de controles de auditoría, control de acceso, integridad y seguridad en la transmisión | Un acuerdo de asociado comercial, análisis de riesgos, políticas, capacitación del personal. El software no lo hace cumplir |
| NHS DTAC / DCB0129/0160 | Borradores de evidencia ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Responsable de seguridad clínica designado, registro de peligros, certificaciones. La evaluación indica que DTAC probablemente no aplica |
| Accesibilidad (WCAG 2.2 AA, sector público) | Verificaciones automáticas y una declaración honesta | Una auditoría manual (T48) antes de cualquier afirmación de cumplimiento |
| Seguridad gubernamental (NIST 800-53, ISO 27001, Cyber Essentials) | Mapeo de controles ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Certificación y aseguramiento independiente |

## Contratos que necesitará y que aún no existen

1. Acuerdo de tratamiento de datos y términos de transferencia internacional con el proveedor de correo (T39).
2. Orden de trabajo y términos de confidencialidad con la firma de pruebas de penetración (T42).
3. Términos de alojamiento y de subencargados para PostgreSQL, Redis y los registros.
4. Licencia: consulte [LICENSE.md](../../../../LICENSE.md); el repositorio también incluye
   declaraciones de seguridad, gobernanza y uso de IA.

## Puntos legales específicos de vigilancia

- La **supresión** anonimiza al contacto y se rechaza mientras exista una negociación, una incidencia
  o una inscripción activa en una secuencia de seguimiento; la cadena de auditoría conserva registros de acciones,
  así que defina en su DPIA cómo interactúa esto con las solicitudes de supresión.
- Las lecturas de datos enmascarados con **acceso de emergencia (break-glass)** son lícitas solo con su base documentada;
  el campo de motivo es evidencia, no autoridad.
- La **puntuación** de prospectos: si alguna vez influye en decisiones significativas sobre
  personas, se aplican las reglas de decisiones automatizadas ([regulatory](../../../../spec/regulatory.md)).
- **Compromisos de la API:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)
  fija un aviso de 180 días antes de retirar un cambio de la API. Es una decisión
  del mantenedor; si los clientes van a depender de ello, decida si debe incluirse
  en un contrato.
- **Declaraciones públicas:** no afirme cumplimiento, certificación ni aprobación.
