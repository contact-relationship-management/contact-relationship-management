# Para el Director de Estrategia

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## La pregunta estratégica

¿Para qué sirve? Hoy es un **CRM de referencia guiado por especificación**, con una capa de seguridad, privacidad y
auditoría inusualmente completa. No es un producto con clientes y no compite con los CRM maduros
([COMPARISONS.md](../../../../COMPARISONS.md)). La decisión que tiene enfrente es
si mantenerlo como referencia o invertir para hacerlo desplegable en entornos
regulados.

## Qué es realmente distinto

| Fortaleza | Por qué podría importar | Qué tan sólida es |
|---|---|---|
| Especificación primero: una sola especificación guía tanto el servicio en Rust como la interfaz en Svelte ([spec](../../../../spec/index.md)) | Más barato de auditar, portar y mantener consistente | Real; las pruebas citan la especificación |
| Marketing que parte del consentimiento, aplicado en la ruta de envío | Un requisito indispensable para compradores del sector salud y público | Real, pero aún no existe un emisor real (T39) |
| Puntuación y pronóstico de prospectos explicables, basados en reglas | Compradores que deben justificar decisiones sobre personas | Real; deliberadamente sin aprendizaje automático |
| Contactos tomados de un servicio de identidad, no duplicados | Menos datos personales guardados, menor superficie de filtración | Real; depende de que ese servicio exista |
| Auditoría a prueba de manipulación, enmascaramiento por rol, acceso de emergencia justificado | Los controles que los compradores regulados preguntan primero | Implementado y probado; no verificado de forma independiente |

## Qué no es distinto

La amplitud de funciones. Cotizaciones, contratos, telefonía, puntuación con aprendizaje automático y
pagos están fuera de alcance ([scope](../../../../spec/scope.md)). Quien elija un
CRM por sus funciones elegirá otro.

## Opciones

| Opción | Qué significa | Costo y riesgo | Evidencia necesaria |
|---|---|---|---|
| A. Seguir como referencia | Mantenerlo como biblioteca de enseñanza y de patrones | El más bajo; el valor es de reputación | Ninguna más allá de la actual |
| B. Nicho del sector regulado | Terminar los pendientes y venderlo o darle soporte a organismos de salud y públicos | El más alto; requiere un proveedor, términos legales, pruebas externas, aseguramiento continuo | Prueba independiente (T42), auditoría de accesibilidad (T48), correo real (T39) |
| C. Componentes, no producto | Ofrecer la cadena de auditoría, el enmascaramiento y los patrones de acceso de emergencia como piezas reutilizables | Moderado; requiere empaquetado y documentación | Trabajo de extracción; sin nueva certificación |

**Recomendación: hacer A ahora y financiar la prueba más barata de B antes de comprometerse.**
La evaluación del NHS concluyó que el producto probablemente queda fuera del alcance de DTAC
([evaluación](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
así que la ruta al mercado no está definida. Invierta primero en averiguar qué comprador
necesita esto y qué exigiría, no en certificaciones que nadie ha pedido.

## Hoja de ruta, en el orden que sugiere la evidencia

1. Prueba de seguridad independiente y auditoría manual de accesibilidad (T42, T48): definen
   si B es siquiera posible.
2. Proveedor de correo real (T39) una vez elegidos un proveedor y los términos de tratamiento de datos;
   el punto de integración del emisor y una ruta SMTP existen y se ejecutan contra un servidor de prueba local, con un enlace de baja firmado, una lista de supresión y un webhook genérico firmado de resultados, pero aún no hay ningún proveedor integrado ([hoja de ruta](../../../../spec/roadmap.md)).
3. Demostrar la firma de versiones con una primera versión etiquetada (T49): requisito básico para
   adquisiciones. El flujo está construido; no se ha ejecutado.
4. Solo entonces las funciones de producto de la hoja de ruta (atribución, territorios,
   relojes de SLA en horario laboral, portal público de base de conocimiento).

## Riesgos estratégicos

- **Afirmar más de lo que es cierto.** La forma más rápida de perder a un comprador regulado es una
  afirmación de cumplimiento que la evidencia no respalda. Consulte el [resumen de GRC](../gobernanza-riesgo-cumplimiento/index.md).
- **Mantenimiento.** El trabajo de seguridad nunca termina: actualizaciones de dependencias, renovación
  de resúmenes criptográficos, rotación de claves, nuevas pruebas. La opción B lo compromete a ello.
- **Dependencia de un servicio de identidad** que está fuera de este repositorio.
- **Un solo mantenedor.** El proyecto tiene un único mantenedor y reconoce que la profundidad
  de la revisión es la de una persona ([MAINTAINERS.md](../../../../MAINTAINERS.md),
  [GOVERNANCE.md](../../../../GOVERNANCE.md)). La opción B necesita a más de una persona
  que pueda responder por el trabajo de seguridad.

## Decisiones para usted

1. Cuál de A, B o C, y la fecha de revisión de esa elección.
2. Quién es el primer comprador objetivo, identificado mediante conversaciones antes de gastar.
3. Si financiar ahora el aseguramiento independiente (consulte el [resumen del Director Ejecutivo](../director-ejecutivo/index.md)).
