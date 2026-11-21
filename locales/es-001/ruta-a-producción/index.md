# Ruta a producción

> Gestión de Relaciones con Contactos no está lista para producción. Esta página explica qué significa producción
> aquí y qué tanto ha avanzado cada parte.

El software funciona con datos sintéticos. El correo se simula a menos que se configure un modo de correo real. No
ingrese datos personales reales.

## Qué significa producción

Una instalación en producción guarda datos de personas reales y les envía mensajes reales. Para este proyecto eso
exige todo lo siguiente:

- **Inicio de sesión real.** Cada solicitud está autenticada y las reglas de acceso corresponden a los roles de la
  organización que lo implementa.
- **Envío lícito.** Cada mensaje tiene consentimiento registrado y una base legal conforme a la ley de cada lugar al
  que se envía, y pasa por un proveedor de correo real que gestiona rebotes, quejas y bajas.
- **Aseguramiento independiente.** Personas ajenas al proyecto han probado la seguridad y la accesibilidad, y los
  hallazgos están corregidos o aceptados.
- **Operación.** Copia de seguridads que se han restaurado, monitoreo, un proceso de incidentes y alguien de guardia.
- **Evidencia de estabilidad.** Ejecuciones largas con carga realista, en un equipo similar al de la instalación, sin
  errores ni lentitud.

La preparación para producción se decide por instalación. Quien implementa puede cerrar algunas compuertas solo para
su propia organización y sus destinatarios. La evaluación técnica en que se basa esta página es
[spec/plan.md](../../../spec/plan.md). Las compuertas abiertas están en [spec/tasks.md](../../../spec/tasks.md).

## Preparación

Cada porcentaje es una estimación hecha el 2026-11-13 a partir de la lista de tareas, no una medición.

- **Autenticación y control de acceso** (70%): Están construidos el inicio de sesión con tokens PASETO o
  Keycloak/OIDC, las reglas de acceso basadas en atributos con verificaciones a nivel de registro y un modo de
  producción que se niega a iniciar sin autenticación. Ninguna instalación real los ha activado con sus propios
  atributos, y la matriz de cumplimiento aún no cubre todos los manejadores (CRM-G1).
- **Entrega de correo** (40%): Un emisor simulado es el predeterminado y existe un emisor SMTP. No hay un adaptador
  de proveedor de correo con webhooks de rebotes y quejas ni una lista de supresión (CRM-T39).
- **Protección de datos y ley de marketing electrónico** (15%): Están construidos los registros de consentimiento, el
  acceso del titular, la supresión y la retención. No se ha leído la ley de ninguna región para este proyecto, y la
  revisión de la ruta real de envío sigue abierta (CRM-G2, [normativa](../../../spec/regulatory.md)).
- **Aseguramiento de seguridad independiente** (10%): Hay un alcance de prueba de penetración redactado. No se ha
  hecho ninguna prueba de penetración, análisis formal de riesgos ni acreditación (CRM-T42). Las auditorías de
  seguridad con IA no son aseguramiento independiente.
- **Accesibilidad** (50%): Las verificaciones automáticas de axe-core contra WCAG 2.2 A y AA se aprueban en cada ruta
  de página. No se ha hecho una auditoría manual con uso solo de teclado, lectores de pantalla ni ampliación al 400%
  (CRM-T48).
- **Traducciones** (30%): La interfaz está disponible en ocho idiomas. Las cadenas en idiomas distintos del inglés
  fueron escritas por una máquina y ningún hablante las ha revisado.
- **Operación** (30%): Un manual de operaciones cubre copia de seguridad y restauración, gestión de claves, cifrado en reposo,
  respuesta a incidentes y retención de registros. No se ha probado ninguna restauración y la lista de verificación
  de puesta en marcha del manual sigue abierta.
- **Rendimiento y estabilidad** (20%): Existen pruebas de carga y una prueba de estabilidad de larga duración. La
  prueba de estabilidad se ha ejecutado durante dos minutos contra la pila de demostración; no hay una ejecución
  larga ni una línea base en equipo similar al de producción.
- **Integridad de las versiones** (85%): Las imágenes de las versiones son multiarquitectura, están firmadas y
  atestiguadas, y sus firmas se verifican. Las acciones y las imágenes están fijadas por resumen criptográfico, y el
  historial se examina en busca de secretos. La mitad amd64 de una imagen de versión descargada no se ha verificado,
  y aún no hay un segundo alojamiento de código (CRM-T192).
- **Integración continua** (100%): Un script ejecuta todas las verificaciones en GitHub Actions y Woodpecker. Los dos
  códigos en Rust y la interfaz tienen un mínimo de 95% de líneas cubiertas; hay compuertas de navegador, lint,
  cadena de suministro, mutación de líneas modificadas y mutación completa semanal (CRM-T94).
- **Salvaguardas de la demostración** (100%): Cada visitante del recorrido recibe una copia independiente de los
  datos de demostración, con un tope de 200 copias. El recorrido está desactivado de forma predeterminada; configure
  `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` para ofrecerlo (CRM-T92).

## Cómo se mantiene esta página

Cuando se cierra una compuerta, cambie la estimación y su evidencia en la misma confirmación (commit) que el trabajo, y
enlace la tarea.
