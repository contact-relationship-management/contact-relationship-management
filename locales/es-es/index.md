# Gestión de Relaciones con Contactos

Software CRM de código abierto para ventas, marketing y atención al cliente.

Gestión de Relaciones con Contactos da seguimiento a las relaciones con clientes y prospectos en ventas, marketing y soporte. Conserva un solo registro por contacto y exige un consentimiento registrado antes de enviar cualquier mensaje. El software no está listo para producción: funciona con datos sintéticos.

## Beneficios

- **Un solo registro por contacto.** Ventas, marketing, soporte y analítica usan el mismo registro.
- **Consentimiento registrado.** No se envía nada a un contacto sin consentimiento registrado. Cada correo incluye un enlace de baja firmado.
- **Menos datos personales.** El CRM guarda una referencia a una persona en un servicio de identidad, no una copia del nombre ni de la dirección postal.
- **Resultados explicables.** Las puntuaciones de prospectos se calculan con reglas legibles. Los pronósticos se calculan a partir de etapas ponderadas del embudo.
- **Diseño internacional.** Los nombres, direcciones postales, calendarios y sentidos de escritura no dependen del formato de un solo país. La interfaz está disponible en ocho idiomas.
- **Especificación pública.** La especificación, el código y las pruebas son públicos. La documentación indica qué está construido y qué no.

## Funciones

- **Ventas.** Puntuación de prospectos basada en reglas, embudos de negociaciones en un tablero y pronósticos ponderados por etapa.
- **Marketing.** Campañas segmentadas y secuencias de seguimiento. El envío es simulado a menos que se configure un modo de correo.
- **Soporte.** Incidencias con prioridades y plazos de nivel de servicio, y artículos de base de conocimiento.
- **Analítica.** Tableros de tasa de cierre, embudo por etapa, retorno de campañas, estado de los niveles de servicio y valor del cliente a lo largo de su vida.
- **Calidad de datos.** Detección de duplicados por probabilidad, con fusión y deshacer.
- **Gobernanza.** Bitácora de auditoría, acceso basado en roles y apoyo para solicitudes de acceso y de supresión de datos de los titulares.

## Primeros pasos

1. **Obtenga el código.** Clone el repositorio. Se requieren Node.js y pnpm.
2. **Inicie la aplicación.** En el directorio de Svelte, ejecute `pnpm install` y luego `pnpm dev`.
3. **Inicie sesión.** Use `example@example.com`. No se requiere ningún otro servicio.
4. **Haga el recorrido.** El recorrido abarca contactos, prospectos, negociaciones, campañas, incidencias y la vista ejecutiva, con datos ficticios.

El [tutorial](../../docs/tutorial-first-tour.md) describe cada paso. [Instalación y uso](../../INSTALL.md) cubre el servicio completo.

## Comparación

| Proyecto | Categoría | Diferencia |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM de código abierto | Producto completo. Este proyecto es una demostración más pequeña, guiada por una especificación. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM de código abierto | Funciones más amplias. Este proyecto destaca las reglas explicables. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM de código abierto | Maduro y grande. Este proyecto es una referencia compacta. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | CRM comercial como servicio | Este proyecto modela la misma práctica. No comparte código ni datos. |

Este proyecto es una demostración y no compite con estos productos. Consulte [todas las comparaciones](../../COMPARISONS.md).

## Resúmenes por rol

Los [resúmenes por rol](para/) ofrecen una página para cada puesto directivo: qué hace el software, qué no hace y qué decisiones corresponden a ese puesto.

## Estado

Este software no está listo para producción ([ruta a producción](ruta-a-producción/index.md)). Use únicamente datos sintéticos. No se envía correo real a menos que se configure un modo de correo real. Nada en este sitio constituye asesoría legal.
