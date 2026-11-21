# Para el Director de Operaciones

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Qué implica operarlo

El servicio es un proceso de Rust por réplica, más PostgreSQL 18 y Redis; la
interfaz es un proceso de Node. Ambos se distribuyen como imágenes de contenedor endurecidas, sin privilegios de root,
con imágenes base fijadas por resumen criptográfico. El manual de operaciones es
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Datos operativos para planear

- **Falla de forma segura al iniciar.** En producción el servicio se niega a iniciar si
  la autenticación está desactivada, si la cadena de auditoría es inutilizable o si faltan las claves requeridas y
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`. Una instalación mal configurada se detiene en lugar de ejecutarse abierta.
- **Salud y métricas.** `/_health/live`, `/_health/ready` y `/_health/startup` nunca tienen
  límite de tasa, de modo que un bloqueo no puede reiniciar un pod sano. Métricas en
  `/metrics.prom`.
- Los **límites contra abuso** están activados en producción: 600 solicitudes por minuto por cliente y un
  bloqueo tras 10 inicios de sesión fallidos en 5 minutos. Se comparten entre
  réplicas mediante Redis; si Redis falla, se recurre a conteos por réplica.
- **Acceso de emergencia** (break-glass): cualquier usuario con sesión iniciada puede tomar una
  elevación de lectura justificada por hasta 60 minutos. Otra persona debe revisar cada concesión.
  **Proceso que usted debe ejecutar:** alertar sobre las filas de auditoría `break_glass_granted` y
  revisar el conteo de pendientes de revisión (página de administración `/break-glass/review`).
- Los **errores** tienen una sola forma (RFC 9457 `application/problem+json`, conservando `error` y
  `description`), de modo que el análisis de registros y las reglas de alerta pueden apoyarse en `status` y `type`.
- **Crecimiento de la auditoría.** Las lecturas se auditan, por lo que la tabla de auditoría crece con el tráfico.
  Dimensione su retención. Un cliente que consulta con frecuencia la hará crecer rápido.

## Trabajo operativo pendiente

| Elemento | Estado |
|---|---|
| Simulacros de respaldo/restauración, ensayo de rotación de claves | Escritos en el manual; **usted** debe ejecutarlos y dejar evidencia |
| Anclaje de la cabeza de la cadena de auditoría fuera de la base de datos | Su responsabilidad; la API informa el hash de la cabeza |
| Actualización de los resúmenes criptográficos de imágenes fijadas | Dependabot abre solicitudes de cambios semanales; **usted** debe revisarlas y fusionarlas o las fijaciones quedan obsoletas |
| Verificación de una versión firmada antes de desplegarla | Los comandos están en el manual; despliegue por resumen criptográfico y rechace una imagen que no supere la verificación. Sin probar hasta una primera versión etiquetada |
| Proveedor de correo real (T39) | Abierto; cambia el panorama de incidentes y proveedores |
| Redis guarda datos de analítica | Protéjalo como la base de datos: TLS, contraseña, sin exposición pública |

## Límites conocidos que conviene comunicar al equipo de guardia

El cierre de sesión por inactividad ocurre en el cliente; un token revocado se rechaza de inmediato, pero una
recarga de políticas se refleja en la analítica en caché hasta 30 segundos después; el acceso de emergencia no puede
revocarse, solo vencer.
